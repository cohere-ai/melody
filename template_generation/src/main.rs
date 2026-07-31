use anyhow::{Context, Result, bail};
use clap::Parser;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
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
#[serde(untagged)]
enum LiquidTemplateConfig {
    RawTemplate(String),
    Config {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        variables: HashMap<String, LiquidTemplateConfig>,
    },
}

impl LiquidTemplateConfig {
    fn get_template_string(&self, base_dir: &str) -> Result<String> {
        match self {
            Self::RawTemplate(s) => Ok(s.clone()),
            Self::Config { path, variables } => {
                let full_path = format!(
                    "{base_dir}/{}",
                    path.as_ref().context("Template path is required but not provided")?
                );
                let content = fs::read_to_string(&full_path)
                    .with_context(|| format!("Failed to read file: {full_path}"))?;
                if variables.is_empty() {
                    return Ok(content);
                }
                let mut keys: Vec<_> = variables.keys().collect();
                keys.sort();
                let captures = keys
                    .into_iter()
                    .map(|key| {
                        let body = variables[key]
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
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JinjaTemplateConfig {
    RawTemplate(String),
    Config {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        includes: HashMap<String, JinjaTemplateConfig>,
    },
}

impl JinjaTemplateConfig {
    fn get_template_string(&self, base_dir: &str) -> Result<String> {
        match self {
            Self::RawTemplate(s) => Ok(s.clone()),
            Self::Config { path, includes } => {
                let full_path = format!(
                    "{base_dir}/{}",
                    path.as_ref().context("Template path is required but not provided")?
                );
                let mut content = fs::read_to_string(&full_path)
                    .with_context(|| format!("Failed to read file: {full_path}"))?;
                for (key, include_config) in includes {
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
    }
}

/// One compiled template revision (jinja and/or liquid bodies).
struct Compiled {
    name: String,
    revision: u32,
    jinja: Option<String>,
    liquid: Option<String>,
}

impl Compiled {
    fn id(&self) -> String {
        format!("{}@{}", self.name, self.revision)
    }

    fn static_name(&self, engine: &str) -> String {
        format!(
            "TPL_{}_{engine}",
            format!("{}_{}", self.name, self.revision)
                .to_uppercase()
                .replace(['-', '/'], "_")
        )
    }

    /// `(archive-relative path, body, file extension for latest symlink)`.
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
    for (key, body, _) in compiled.iter().flat_map(Compiled::artifacts) {
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
    if !conflicts.is_empty() {
        for c in &conflicts {
            eprintln!("  - {c}");
        }
        bail!("template content changed without bumping revision");
    }
    if dirty {
        fs::create_dir_all("gen")?;
        fs::write(LOCK_PATH, format!("{}\n", serde_json::to_string_pretty(&locks)?))?;
        println!("Updated {LOCK_PATH}");
    }
    Ok(())
}

fn write_archive_file(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        let existing = fs::read_to_string(path)?;
        if existing == content {
            return Ok(());
        }
        bail!(
            "{} already exists with different content; bump `revision`",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn write_latest_symlink(dir: &Path, link_name: &str, target: &str) -> Result<()> {
    fs::create_dir_all(dir)?;
    let link = dir.join(link_name);
    if link
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
        && fs::read_link(&link).ok().as_deref() == Some(Path::new(target))
    {
        return Ok(());
    }
    let _ = fs::remove_file(&link);
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, &link)
        .with_context(|| format!("symlink {} -> {target}", link.display()))?;
    #[cfg(not(unix))]
    bail!("latest symlinks require unix ({})", link.display());
    Ok(())
}

fn write_archive(compiled: &[Compiled]) -> Result<()> {
    let root = PathBuf::from("gen/templates/archive");
    fs::create_dir_all(&root)?;
    for t in compiled {
        for (rel, body, ext) in t.artifacts() {
            write_archive_file(&root.join(&rel), body)?;
            write_latest_symlink(
                &root.join(&t.name),
                &format!("latest.{ext}"),
                &format!("{}@{}.{}", t.name, t.revision, ext),
            )?;
        }
    }
    Ok(())
}

fn write_embeds(compiled: &[Compiled]) -> Result<()> {
    let mut out = String::from(
        "// @generated by template_generation. Do not edit by hand.\n\
         // Build config: template_generation/template_registry.yaml\n\n\
         #![allow(dead_code)]\n#![allow(clippy::all)]\n\n",
    );

    for t in compiled {
        if t.jinja.is_some() {
            let path = format!("templates/archive/{0}/{0}@{1}.jinja", t.name, t.revision);
            out.push_str(&format!(
                "pub static {}: &str = include_str!({path:?});\n",
                t.static_name("JINJA")
            ));
        }
        if t.liquid.is_some() {
            let path = format!("templates/archive/{0}/{0}@{1}.tmpl", t.name, t.revision);
            out.push_str(&format!(
                "pub static {}: &str = include_str!({path:?});\n",
                t.static_name("LIQUID")
            ));
        }
    }

    let emit_lookup = |out: &mut String, fn_name: &str, engine: &str, items: &[&Compiled]| {
        out.push_str(&format!(
            "\n/// Look up an embedded {engine} template by id (`{{name}}` or `{{name}}@{{revision}}`).\n\
             pub fn {fn_name}(id: &str) -> Option<&'static str> {{\n    match id {{\n"
        ));
        for t in items {
            out.push_str(&format!(
                "        {:?} | {:?} => Some({}),\n",
                t.name,
                t.id(),
                t.static_name(&engine.to_uppercase())
            ));
        }
        out.push_str("        _ => None,\n    }\n}\n");
    };

    let jinja: Vec<_> = compiled.iter().filter(|t| t.jinja.is_some()).collect();
    let liquid: Vec<_> = compiled.iter().filter(|t| t.liquid.is_some()).collect();
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
    enforce_locks(&compiled)?;
    write_archive(&compiled)?;
    write_embeds(&compiled)?;
    println!(
        "Generated {} templates from {}",
        compiled.len(),
        args.registry
    );
    Ok(())
}
