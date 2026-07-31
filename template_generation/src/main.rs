use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "template_generation/template_registry.yaml")]
    registry: String,
    #[arg(long, default_value = "template_generation/templates/jinja")]
    jinja_templates_dir: String,
    #[arg(long, default_value = "template_generation/templates/liquid")]
    liquid_templates_dir: String,
}

#[derive(Debug, Deserialize)]
struct Registry {
    templates: BTreeMap<String, TemplateConfig>,
}

#[derive(Debug, Deserialize)]
struct TemplateConfig {
    revision: u32,
    entry: EntryConfig,
}

#[derive(Debug, Deserialize)]
struct EntryConfig {
    #[serde(default)]
    jinja: Option<JinjaTemplateConfig>,
    #[serde(default)]
    liquid: Option<LiquidTemplateConfig>,
}

#[derive(Debug, Deserialize)]
struct LiquidTemplateConfig {
    path: String,
    #[serde(default)]
    variables: HashMap<String, LiquidTemplateConfig>,
}

impl LiquidTemplateConfig {
    fn get_template_string(&self, base_dir: &str) -> Result<String> {
        let full_path = format!("{base_dir}/{}", self.path);
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {full_path}"))?;
        if self.variables.is_empty() {
            return Ok(content);
        }
        let mut keys: Vec<_> = self.variables.keys().collect();
        keys.sort();
        let captures = keys
            .into_iter()
            .map(|key| {
                let body = self.variables[key]
                    .get_template_string(base_dir)?
                    .trim_end_matches('\n')
                    .to_string();
                Ok(format!("{{% capture {key} %}}{body}{{% endcapture %}}"))
            })
            .collect::<Result<Vec<_>>>()?
            .join("");
        Ok(format!("{captures}{content}"))
    }
}

#[derive(Debug, Deserialize)]
struct JinjaTemplateConfig {
    path: String,
    #[serde(default)]
    includes: HashMap<String, JinjaTemplateConfig>,
}

impl JinjaTemplateConfig {
    fn get_template_string(&self, base_dir: &str) -> Result<String> {
        let full_path = format!("{base_dir}/{}", self.path);
        let mut content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {full_path}"))?;
        for (key, include_config) in &self.includes {
            let include_content = include_config
                .get_template_string(base_dir)?
                .trim_end_matches('\n')
                .to_string();
            for pattern in [
                format!(r#"\{{%\s*include\s*"{key}"\s*%\}}"#),
                format!(r#"\s*\{{%-\s*include\s*"{key}"\s*%\}}\s*"#),
            ] {
                let re = regex::Regex::new(&pattern)
                    .with_context(|| format!("Failed to create regex for include: {key}"))?;
                content = re
                    .replace_all(&content, regex::NoExpand(include_content.as_str()))
                    .into_owned();
            }
        }
        Ok(content)
    }
}

struct Compiled {
    name: String,
    revision: u32,
    jinja: Option<String>,
    liquid: Option<String>,
}

impl Compiled {
    /// `(archive-relative path, body, extension for latest copy)`.
    fn artifacts(&self) -> Vec<(String, &str, &str)> {
        let mut out = Vec::new();
        if let Some(body) = &self.jinja {
            out.push((
                format!("{0}/{0}@{1}.jinja", self.name, self.revision),
                body.as_str(),
                "jinja",
            ));
        }
        if let Some(body) = &self.liquid {
            out.push((
                format!("{0}/{0}@{1}.tmpl", self.name, self.revision),
                body.as_str(),
                "tmpl",
            ));
        }
        out
    }
}

fn compile(registry: &Registry, args: &Args) -> Result<Vec<Compiled>> {
    let mut compiled = Vec::new();
    for (name, template) in &registry.templates {
        if name.contains('@') {
            bail!("template name '{name}' must not contain '@'");
        }
        let jinja = template
            .entry
            .jinja
            .as_ref()
            .map(|c| c.get_template_string(&args.jinja_templates_dir))
            .transpose()
            .with_context(|| format!("jinja {name}"))?;
        let liquid = template
            .entry
            .liquid
            .as_ref()
            .map(|c| c.get_template_string(&args.liquid_templates_dir))
            .transpose()
            .with_context(|| format!("liquid {name}"))?;
        if jinja.is_none() && liquid.is_none() {
            bail!("{name}: entry must include jinja and/or liquid");
        }
        compiled.push(Compiled {
            name: name.clone(),
            revision: template.revision,
            jinja,
            liquid,
        });
    }
    compiled.sort_by(|a, b| (&a.name, a.revision).cmp(&(&b.name, b.revision)));
    Ok(compiled)
}

fn sha256(content: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(content.as_bytes()))
}

fn enforce_locks(compiled: &[Compiled]) -> Result<()> {
    const LOCK_PATH: &str = "gen/template_revision_locks.json";
    let archive_root = PathBuf::from("gen/templates/archive");
    let mut locks: BTreeMap<String, String> = Path::new(LOCK_PATH)
        .exists()
        .then(|| fs::read_to_string(LOCK_PATH).ok())
        .flatten()
        .map(|raw| serde_json::from_str(&raw))
        .transpose()
        .with_context(|| format!("Failed to parse {LOCK_PATH}"))?
        .unwrap_or_default();

    let mut conflicts = Vec::new();
    let mut dirty = false;
    let mut active_keys = HashSet::new();

    // Current revisions compiled from the registry.
    for (key, body, _) in compiled.iter().flat_map(Compiled::artifacts) {
        active_keys.insert(key.clone());
        let hash = sha256(body);
        match locks.get(&key) {
            Some(existing) if existing != &hash => conflicts.push(format!(
                "{key}: content changed (locked {existing}, got {hash}). Bump `revision`."
            )),
            Some(_) => {}
            None => {
                locks.insert(key, hash);
                dirty = true;
            }
        }
    }

    // Frozen older archive revisions (raw files only — not rebuilt from YAML).
    for t in compiled {
        let dir = archive_root.join(&t.name);
        if !dir.is_dir() {
            continue;
        }
        for ent in fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
            let ent = ent?;
            let fname = ent.file_name();
            let fname = fname.to_string_lossy();
            let Some((rev, _)) = parse_archive_filename(&t.name, &fname) else {
                continue;
            };
            if rev == t.revision {
                continue; // current — locked from compiled body above
            }
            let key = format!("{}/{fname}", t.name);
            active_keys.insert(key.clone());
            let body = fs::read_to_string(ent.path())
                .with_context(|| format!("read frozen {}", ent.path().display()))?;
            let hash = sha256(&body);
            match locks.get(&key) {
                Some(existing) if existing != &hash => conflicts.push(format!(
                    "{key}: frozen archive content changed (locked {existing}, got {hash}). \
                     Restore the file or remove it from the archive."
                )),
                Some(_) => {}
                None => {
                    locks.insert(key, hash);
                    dirty = true;
                }
            }
        }
    }

    let before = locks.len();
    locks.retain(|k, _| active_keys.contains(k));
    if locks.len() != before {
        dirty = true;
    }

    if !conflicts.is_empty() {
        for c in &conflicts {
            eprintln!("  - {c}");
        }
        bail!("template content changed without bumping revision");
    }
    if dirty {
        fs::create_dir_all("gen")?;
        fs::write(
            LOCK_PATH,
            format!("{}\n", serde_json::to_string_pretty(&locks)?),
        )?;
        println!("Updated {LOCK_PATH}");
    }
    Ok(())
}

/// Parse `{name}@{revision}.{jinja|tmpl}` → (revision, ext).
fn parse_archive_filename(name: &str, fname: &str) -> Option<(u32, String)> {
    let prefix = format!("{name}@");
    let rest = fname.strip_prefix(&prefix)?;
    let (rev_str, ext) = rest.rsplit_once('.')?;
    if ext != "jinja" && ext != "tmpl" {
        return None;
    }
    let rev: u32 = rev_str.parse().ok()?;
    Some((rev, ext.to_string()))
}

fn write_archive(compiled: &[Compiled]) -> Result<()> {
    let root = PathBuf::from("gen/templates/archive");
    fs::create_dir_all(&root)?;
    for t in compiled {
        let dir = root.join(&t.name);
        for (rel, body, ext) in t.artifacts() {
            let path = root.join(&rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, body)
                .with_context(|| format!("Failed to write {}", path.display()))?;
            // Real file (not a symlink) so GitHub raw / curl serve the template body.
            let latest = dir.join(format!("latest.{ext}"));
            let _ = fs::remove_file(&latest);
            fs::write(&latest, body)
                .with_context(|| format!("Failed to write {}", latest.display()))?;
        }
        // Drop latest.* for engines the current revision no longer ships.
        if t.jinja.is_none() {
            let _ = fs::remove_file(dir.join("latest.jinja"));
        }
        if t.liquid.is_none() {
            let _ = fs::remove_file(dir.join("latest.tmpl"));
        }
    }
    Ok(())
}

/// Discover `{name}@{rev}.{ext}` files on disk for embed generation.
fn archive_revisions(name: &str, ext: &str) -> Result<Vec<u32>> {
    let dir = PathBuf::from("gen/templates/archive").join(name);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut revs = Vec::new();
    for ent in fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let ent = ent?;
        let fname = ent.file_name();
        let fname = fname.to_string_lossy();
        if let Some((rev, e)) = parse_archive_filename(name, &fname) {
            if e == ext {
                revs.push(rev);
            }
        }
    }
    revs.sort_unstable();
    revs.dedup();
    Ok(revs)
}

fn write_embeds(compiled: &[Compiled]) -> Result<()> {
    let mut out = String::from(
        "// @generated by template_generation. Do not edit by hand.\n\
         // Build config: template_generation/template_registry.yaml\n\n\
         #![allow(dead_code)]\n#![allow(clippy::all)]\n",
    );

    // (is_current, name, id, path)
    let collect = |ext: &str| -> Result<Vec<(bool, String, String, String)>> {
        let mut items = Vec::new();
        for t in compiled {
            let current = t.revision;
            for rev in archive_revisions(&t.name, ext)? {
                let id = format!("{}@{rev}", t.name);
                let path = format!("templates/archive/{0}/{0}@{rev}.{ext}", t.name);
                items.push((rev == current, t.name.clone(), id, path));
            }
        }
        Ok(items)
    };

    let emit_lookup = |out: &mut String,
                       fn_name: &str,
                       engine: &str,
                       items: &[(bool, String, String, String)]| {
        out.push_str(&format!(
            "\n/// Look up an embedded {engine} template by id (`{{name}}` or `{{name}}@{{revision}}`).\n\
             pub fn {fn_name}(id: &str) -> Option<&'static str> {{\n    match id {{\n"
        ));
        for (is_current, name, id, path) in items {
            if *is_current {
                out.push_str(&format!(
                    "        {name:?} | {id:?} => Some(include_str!({path:?})),\n"
                ));
            } else {
                out.push_str(&format!(
                    "        {id:?} => Some(include_str!({path:?})),\n"
                ));
            }
        }
        out.push_str("        _ => None,\n    }\n}\n");
    };

    let jinja = collect("jinja")?;
    let liquid = collect("tmpl")?;
    emit_lookup(&mut out, "lookup_jinja", "jinja", &jinja);
    emit_lookup(&mut out, "lookup_liquid", "liquid", &liquid);

    fs::write("gen/embedded_templates.rs", out)?;
    Ok(())
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("Error: {e:?}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let registry: Registry = serde_yaml::from_str(
        &fs::read_to_string(&args.registry)
            .with_context(|| format!("Failed to read {}", args.registry))?,
    )
    .with_context(|| format!("Failed to parse {}", args.registry))?;

    let compiled = compile(&registry, &args)?;
    // Lock current (from compiled bodies) and any frozen older archive files on disk
    // before writing, so conflicts never clobber the archive.
    enforce_locks(&compiled)?;
    write_archive(&compiled)?;
    write_embeds(&compiled)?;
    println!(
        "Generated {} current templates from {}",
        compiled.len(),
        args.registry
    );
    Ok(())
}
