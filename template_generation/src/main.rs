use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const REGISTRY: &str = "template_generation/template_registry.yaml";
const JINJA_DIR: &str = "template_generation/templates/jinja";
const LIQUID_DIR: &str = "template_generation/templates/liquid";
const ARCHIVE: &str = "gen/templates/archive";
const LOCKS: &str = "gen/template_revision_locks.json";

#[derive(Debug, Deserialize)]
struct Registry {
    templates: BTreeMap<String, TemplateConfig>,
}

#[derive(Debug, Deserialize)]
struct TemplateConfig {
    revision: u32,
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
    fn render(&self, base_dir: &str) -> Result<String> {
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
                    .render(base_dir)?
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
    fn render(&self, base_dir: &str) -> Result<String> {
        let full_path = format!("{base_dir}/{}", self.path);
        let mut content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {full_path}"))?;
        for (key, include) in &self.includes {
            let body = include.render(base_dir)?.trim_end_matches('\n').to_string();
            for pattern in [
                format!(r#"\{{%\s*include\s*"{key}"\s*%\}}"#),
                format!(r#"\s*\{{%-\s*include\s*"{key}"\s*%\}}\s*"#),
            ] {
                let re = regex::Regex::new(&pattern)
                    .with_context(|| format!("Failed to create regex for include: {key}"))?;
                content = re
                    .replace_all(&content, regex::NoExpand(body.as_str()))
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
    /// Archive-relative path + body for each engine this revision ships.
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

fn compile(registry: &Registry) -> Result<Vec<Compiled>> {
    let mut compiled = Vec::new();
    for (name, template) in &registry.templates {
        if name.contains('@') {
            bail!("template name '{name}' must not contain '@'");
        }
        let jinja = template
            .jinja
            .as_ref()
            .map(|c| c.render(JINJA_DIR))
            .transpose()
            .with_context(|| format!("jinja {name}"))?;
        let liquid = template
            .liquid
            .as_ref()
            .map(|c| c.render(LIQUID_DIR))
            .transpose()
            .with_context(|| format!("liquid {name}"))?;
        if jinja.is_none() && liquid.is_none() {
            bail!("{name}: must include jinja and/or liquid");
        }
        compiled.push(Compiled {
            name: name.clone(),
            revision: template.revision,
            jinja,
            liquid,
        });
    }
    compiled.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(compiled)
}

fn sha256(content: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(content.as_bytes()))
}

/// Lock `key` to `hash`. Records conflicts / new keys on the accumulators.
fn check_lock(
    locks: &mut BTreeMap<String, String>,
    key: String,
    hash: String,
    conflict_hint: &str,
    conflicts: &mut Vec<String>,
    dirty: &mut bool,
    active: &mut HashSet<String>,
) {
    active.insert(key.clone());
    match locks.get(&key) {
        Some(existing) if existing != &hash => {
            conflicts.push(format!(
                "{key}: content changed ({conflict_hint}). Locked {existing}, got {hash}."
            ));
        }
        Some(_) => {}
        None => {
            locks.insert(key, hash);
            *dirty = true;
        }
    }
}

/// `{name}@{revision}.{jinja|tmpl}` → (revision, ext).
fn parse_archive_filename(name: &str, fname: &str) -> Option<(u32, String)> {
    let rest = fname.strip_prefix(&format!("{name}@"))?;
    let (rev_str, ext) = rest.rsplit_once('.')?;
    if ext != "jinja" && ext != "tmpl" {
        return None;
    }
    Some((rev_str.parse().ok()?, ext.to_string()))
}

/// On-disk `{name}@{N}.{ext}` files under the archive dir for `name`.
fn list_archive_files(name: &str) -> Result<Vec<(u32, String, PathBuf)>> {
    let dir = PathBuf::from(ARCHIVE).join(name);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for ent in fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let ent = ent?;
        let fname = ent.file_name();
        let fname = fname.to_string_lossy();
        let Some((rev, ext)) = parse_archive_filename(name, &fname) else {
            continue;
        };
        out.push((rev, ext, ent.path()));
    }
    out.sort_by(|a, b| (&a.1, a.0).cmp(&(&b.1, b.0)));
    Ok(out)
}

fn enforce_locks(compiled: &[Compiled]) -> Result<()> {
    let mut locks: BTreeMap<String, String> = Path::new(LOCKS)
        .exists()
        .then(|| fs::read_to_string(LOCKS).ok())
        .flatten()
        .map(|raw| serde_json::from_str(&raw))
        .transpose()
        .with_context(|| format!("Failed to parse {LOCKS}"))?
        .unwrap_or_default();

    let mut conflicts = Vec::new();
    let mut dirty = false;
    let mut active = HashSet::new();

    for (key, body, _) in compiled.iter().flat_map(Compiled::artifacts) {
        check_lock(
            &mut locks,
            key,
            sha256(body),
            "bump `revision`",
            &mut conflicts,
            &mut dirty,
            &mut active,
        );
    }

    // Frozen older revisions: lock raw archive bytes (not rebuilt from YAML).
    for t in compiled {
        for (rev, ext, path) in list_archive_files(&t.name)? {
            if rev == t.revision {
                continue;
            }
            let key = format!("{}/{}@{}.{}", t.name, t.name, rev, ext);
            let body = fs::read_to_string(&path)
                .with_context(|| format!("read frozen {}", path.display()))?;
            check_lock(
                &mut locks,
                key,
                sha256(&body),
                "restore or delete the archive file",
                &mut conflicts,
                &mut dirty,
                &mut active,
            );
        }
    }

    let before = locks.len();
    locks.retain(|k, _| active.contains(k));
    dirty |= locks.len() != before;

    if !conflicts.is_empty() {
        for c in &conflicts {
            eprintln!("  - {c}");
        }
        bail!("template content changed without bumping revision");
    }
    if dirty {
        fs::create_dir_all("gen")?;
        fs::write(
            LOCKS,
            format!("{}\n", serde_json::to_string_pretty(&locks)?),
        )?;
        println!("Updated {LOCKS}");
    }
    Ok(())
}

fn write_archive(compiled: &[Compiled]) -> Result<()> {
    let root = PathBuf::from(ARCHIVE);
    for t in compiled {
        let dir = root.join(&t.name);
        fs::create_dir_all(&dir)?;
        for (rel, body, ext) in t.artifacts() {
            let path = root.join(&rel);
            fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
            // Real file (not a symlink) so GitHub raw / curl serve the body.
            let latest = dir.join(format!("latest.{ext}"));
            let _ = fs::remove_file(&latest);
            fs::write(&latest, body).with_context(|| format!("write {}", latest.display()))?;
        }
        if t.jinja.is_none() {
            let _ = fs::remove_file(dir.join("latest.jinja"));
        }
        if t.liquid.is_none() {
            let _ = fs::remove_file(dir.join("latest.tmpl"));
        }
    }
    Ok(())
}

fn write_embeds(compiled: &[Compiled]) -> Result<()> {
    let mut out = String::from(
        "// @generated by template_generation. Do not edit by hand.\n\
         // Build config: template_generation/template_registry.yaml\n",
    );

    for (fn_name, engine, ext) in [
        ("lookup_jinja", "jinja", "jinja"),
        ("lookup_liquid", "liquid", "tmpl"),
    ] {
        out.push_str(&format!(
            "\n/// Look up an embedded {engine} template by id (`{{name}}` or `{{name}}@{{revision}}`).\n\
             pub fn {fn_name}(id: &str) -> Option<&'static str> {{\n    match id {{\n"
        ));
        for t in compiled {
            for (rev, file_ext, _) in list_archive_files(&t.name)? {
                if file_ext != ext {
                    continue;
                }
                let id = format!("{}@{rev}", t.name);
                let path = format!("templates/archive/{0}/{0}@{rev}.{ext}", t.name);
                if rev == t.revision {
                    out.push_str(&format!(
                        "        {0:?} | {id:?} => Some(include_str!({path:?})),\n",
                        t.name
                    ));
                } else {
                    out.push_str(&format!(
                        "        {id:?} => Some(include_str!({path:?})),\n"
                    ));
                }
            }
        }
        out.push_str("        _ => None,\n    }\n}\n");
    }

    fs::write("gen/embedded_templates.rs", out)?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let registry: Registry = serde_yaml::from_str(
        &fs::read_to_string(REGISTRY).with_context(|| format!("Failed to read {REGISTRY}"))?,
    )
    .with_context(|| format!("Failed to parse {REGISTRY}"))?;

    let compiled = compile(&registry)?;
    enforce_locks(&compiled)?;
    write_archive(&compiled)?;
    write_embeds(&compiled)?;
    println!(
        "Generated {} current templates from {REGISTRY}",
        compiled.len()
    );
    Ok(())
}
