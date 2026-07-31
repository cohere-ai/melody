use anyhow::{Context, Result, bail};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Map, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Unified template registry
    #[arg(long, default_value = "template_generation/template_registry.yaml")]
    registry: String,
    /// Jinja templates source directory
    #[arg(long, default_value = "template_generation/templates/jinja")]
    jinja_templates_dir: String,
    /// Liquid templates source directory
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
    #[serde(default)]
    deprecated: bool,
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
            LiquidTemplateConfig::RawTemplate(s) => Ok(s.clone()),
            LiquidTemplateConfig::Config { path, variables } => {
                let path_str = path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Template path is required but not provided"))?;
                let full_path = format!("{base_dir}/{path_str}");
                let content = fs::read_to_string(&full_path)
                    .with_context(|| format!("Failed to read file: {full_path}"))?;

                if !variables.is_empty() {
                    let mut variable_strings = Vec::new();
                    let mut keys: Vec<_> = variables.keys().collect();
                    keys.sort();

                    for key in keys {
                        let var_content = variables[key].get_template_string(base_dir)?;
                        let var_content_trimmed = var_content.trim_end_matches('\n');
                        variable_strings.push(format!(
                            "{{% capture {key} %}}{var_content_trimmed}{{% endcapture %}}"
                        ));
                    }

                    let variables_string = variable_strings.join("");
                    Ok(format!("{variables_string}{content}"))
                } else {
                    Ok(content)
                }
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
            JinjaTemplateConfig::RawTemplate(s) => Ok(s.clone()),
            JinjaTemplateConfig::Config { path, includes } => {
                let path_str = path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Template path is required but not provided"))?;
                let full_path = format!("{base_dir}/{path_str}");
                let mut content = fs::read_to_string(&full_path)
                    .with_context(|| format!("Failed to read file: {full_path}"))?;

                if !includes.is_empty() {
                    for (key, include_config) in includes {
                        let include_content = include_config
                            .get_template_string(base_dir)?
                            .trim_end_matches('\n')
                            .to_string();
                        let include_statement = format!(r#"\{{%\s*include\s*"{key}"\s*%\}}"#);
                        let re = regex::Regex::new(&include_statement).with_context(|| {
                            format!("Failed to create regex for include: {key}")
                        })?;
                        content = re
                            .replace_all(&content, regex::NoExpand(include_content.as_str()))
                            .to_string();

                        let include_statement_trim =
                            format!(r#"\s*\{{%-\s*include\s*"{key}"\s*%\}}\s*"#);
                        let re = regex::Regex::new(&include_statement_trim).with_context(|| {
                            format!("Failed to create regex for include: {key}")
                        })?;
                        content = re
                            .replace_all(&content, regex::NoExpand(include_content.as_str()))
                            .to_string();
                    }
                }
                Ok(content)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct CompiledTemplate {
    name: String,
    revision: u32,
    deprecated: bool,
    jinja: Option<String>,
    liquid: Option<String>,
}

impl CompiledTemplate {
    fn canonical_id(&self) -> String {
        format!("{}@{}", self.name, self.revision)
    }

    fn archive_jinja_rel(&self) -> String {
        format!("{0}/{0}@{1}.jinja", self.name, self.revision)
    }

    fn archive_liquid_rel(&self) -> String {
        format!("{0}/{0}@{1}.tmpl", self.name, self.revision)
    }

    fn embed_jinja_rel(&self) -> String {
        format!("templates/archive/{}", self.archive_jinja_rel())
    }

    fn embed_liquid_rel(&self) -> String {
        format!("templates/archive/{}", self.archive_liquid_rel())
    }

    fn static_name(&self, engine: &str) -> String {
        let name = self.name.to_uppercase().replace(['-', '/'], "_");
        format!("TPL_{name}_{}_{engine}", self.revision)
    }
}

fn compile_registry(registry: &Registry, args: &Args) -> Result<Vec<CompiledTemplate>> {
    let mut compiled = Vec::new();

    for (name, template) in &registry.templates {
        if name.contains('@') {
            bail!("template name '{name}' must not contain '@'");
        }

        let jinja = if let Some(cfg) = template.entry.jinja.as_ref() {
            Some(
                cfg.get_template_string(&args.jinja_templates_dir)
                    .with_context(|| format!("jinja {name}"))?,
            )
        } else {
            None
        };

        let liquid = if let Some(cfg) = template.entry.liquid.as_ref() {
            Some(
                cfg.get_template_string(&args.liquid_templates_dir)
                    .with_context(|| format!("liquid {name}"))?,
            )
        } else {
            None
        };

        if jinja.is_none() && liquid.is_none() {
            bail!("{name}: entry must include jinja and/or liquid");
        }

        compiled.push(CompiledTemplate {
            name: name.clone(),
            revision: template.revision,
            deprecated: template.deprecated,
            jinja,
            liquid,
        });
    }

    compiled.sort_by(|a, b| (&a.name, a.revision).cmp(&(&b.name, b.revision)));
    Ok(compiled)
}

fn write_outputs(compiled: &[CompiledTemplate]) -> Result<()> {
    // Append-only archive: prior revisions stay on disk when revision is bumped.
    // Melody embeds pinned `@N` files via include_str!; `latest` symlinks float.
    fs::create_dir_all("gen/templates/archive")?;

    for v in compiled {
        if let Some(jinja) = &v.jinja {
            let archive = format!("gen/templates/archive/{}", v.archive_jinja_rel());
            write_archive_file(&archive, jinja)?;
            write_latest_symlink(
                &format!("gen/templates/archive/{}", v.name),
                "latest.jinja",
                &format!("{}@{}.jinja", v.name, v.revision),
            )?;
        }

        if let Some(liquid) = &v.liquid {
            let archive = format!("gen/templates/archive/{}", v.archive_liquid_rel());
            write_archive_file(&archive, liquid)?;
            write_latest_symlink(
                &format!("gen/templates/archive/{}", v.name),
                "latest.tmpl",
                &format!("{}@{}.tmpl", v.name, v.revision),
            )?;
        }
    }

    Ok(())
}

/// Write an immutable archive revision file. Existing bytes must match (locks
/// already enforce this); identical content is a no-op so git stays quiet.
fn write_archive_file(path: &str, content: &str) -> Result<()> {
    if Path::new(path).exists() {
        let existing = fs::read_to_string(path)
            .with_context(|| format!("Failed to read existing {path}"))?;
        if existing == content {
            return Ok(());
        }
        bail!(
            "{path} already exists with different content; bump `revision` in template_registry.yaml"
        );
    }
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content).with_context(|| format!("Failed to write {path}"))?;
    Ok(())
}

/// Create or update a relative symlink named `link_name` in `dir`.
fn write_latest_symlink(dir: &str, link_name: &str, relative_target: &str) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("Failed to create {dir}"))?;
    let link = Path::new(dir).join(link_name);

    if link.symlink_metadata().is_ok() {
        if link
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
        {
            let current = fs::read_link(&link)
                .with_context(|| format!("Failed to read symlink {}", link.display()))?;
            if current == Path::new(relative_target) {
                return Ok(());
            }
        }
        fs::remove_file(&link)
            .with_context(|| format!("Failed to remove {}", link.display()))?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(relative_target, &link).with_context(|| {
            format!(
                "Failed to symlink {} -> {relative_target}",
                link.display()
            )
        })?;
    }
    #[cfg(not(unix))]
    {
        bail!(
            "template archive latest symlinks require unix (failed for {})",
            link.display()
        );
    }

    Ok(())
}

fn content_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

/// Enforce immutability of `{name}@{revision}` contents.
///
/// `gen/template_revision_locks.json` maps each archive relative path to
/// a content hash. If the same path is regenerated with different bytes, generation
/// fails and the revision must be bumped in `template_registry.yaml`.
/// Old revision files under `gen/templates/archive/` are left in place (append-only).
fn enforce_revision_locks(compiled: &[CompiledTemplate]) -> Result<()> {
    const LOCK_PATH: &str = "gen/template_revision_locks.json";

    let mut locks: BTreeMap<String, String> = if Path::new(LOCK_PATH).exists() {
        let raw = fs::read_to_string(LOCK_PATH)
            .with_context(|| format!("Failed to read {LOCK_PATH}"))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse {LOCK_PATH}"))?
    } else {
        BTreeMap::new()
    };

    let mut conflicts = Vec::new();
    let mut additions = BTreeMap::new();

    for v in compiled {
        if let Some(jinja) = &v.jinja {
            let key = v.archive_jinja_rel();
            let hash = content_sha256(jinja);
            match locks.get(&key) {
                Some(existing) if existing != &hash => {
                    conflicts.push(format!(
                        "{key}: content changed for existing revision (locked {existing}, got {hash}). Bump `revision` in template_registry.yaml."
                    ));
                }
                Some(_) => {}
                None => {
                    additions.insert(key, hash);
                }
            }
        }
        if let Some(liquid) = &v.liquid {
            let key = v.archive_liquid_rel();
            let hash = content_sha256(liquid);
            match locks.get(&key) {
                Some(existing) if existing != &hash => {
                    conflicts.push(format!(
                        "{key}: content changed for existing revision (locked {existing}, got {hash}). Bump `revision` in template_registry.yaml."
                    ));
                }
                Some(_) => {}
                None => {
                    additions.insert(key, hash);
                }
            }
        }
    }

    if !conflicts.is_empty() {
        eprintln!("Revision lock conflicts ({}):", conflicts.len());
        for c in &conflicts {
            eprintln!("  - {c}");
        }
        bail!(
            "template content changed without bumping revision; see conflicts above"
        );
    }

    if !additions.is_empty() {
        locks.extend(additions);
        if let Some(parent) = Path::new(LOCK_PATH).parent() {
            fs::create_dir_all(parent)?;
        }
        let pretty = serde_json::to_string_pretty(&locks)?;
        fs::write(LOCK_PATH, format!("{pretty}\n"))
            .with_context(|| format!("Failed to write {LOCK_PATH}"))?;
        println!("Updated {LOCK_PATH}");
    }

    Ok(())
}

fn rust_string_literal(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn write_rust_registry(compiled: &[CompiledTemplate]) -> Result<()> {
    let mut out = String::new();
    out.push_str("// @generated by template_generation. Do not edit by hand.\n");
    out.push_str("// Source: template_generation/template_registry.yaml\n\n");
    out.push_str("#![allow(dead_code)]\n");
    out.push_str("#![allow(clippy::all)]\n\n");

    out.push_str("/// Metadata for a registered template revision.\n");
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    out.push_str("pub struct TemplateMeta {\n");
    out.push_str("    /// Canonical id: `{name}@{revision}`.\n");
    out.push_str("    pub id: &'static str,\n");
    out.push_str("    /// Template name (e.g. cmd4-reasoning).\n");
    out.push_str("    pub name: &'static str,\n");
    out.push_str("    /// Monotonic revision within the name.\n");
    out.push_str("    pub revision: u32,\n");
    out.push_str("    /// Whether this revision is deprecated.\n");
    out.push_str("    pub deprecated: bool,\n");
    out.push_str("    /// Path-addressable archive relative path (jinja), if any.\n");
    out.push_str("    pub archive_path_jinja: Option<&'static str>,\n");
    out.push_str("    /// Path-addressable archive relative path (liquid), if any.\n");
    out.push_str("    pub archive_path_liquid: Option<&'static str>,\n");
    out.push_str("}\n\n");

    out.push_str("/// A resolved template with optional engine bodies.\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct ResolvedTemplate {\n");
    out.push_str("    /// Registry metadata for the resolved revision.\n");
    out.push_str("    pub meta: TemplateMeta,\n");
    out.push_str("    /// Flattened jinja body, when this revision has a jinja engine.\n");
    out.push_str("    pub jinja: Option<&'static str>,\n");
    out.push_str("    /// Flattened liquid body, when this revision has a liquid engine.\n");
    out.push_str("    pub liquid: Option<&'static str>,\n");
    out.push_str("}\n\n");

    for v in compiled {
        if v.jinja.is_some() {
            let name = v.static_name("JINJA");
            let path = v.embed_jinja_rel();
            out.push_str(&format!(
                "pub static {name}: &str = include_str!({path:?});\n"
            ));
        }
        if v.liquid.is_some() {
            let name = v.static_name("LIQUID");
            let path = v.embed_liquid_rel();
            out.push_str(&format!(
                "pub static {name}: &str = include_str!({path:?});\n"
            ));
        }
    }
    out.push('\n');

    out.push_str("const TEMPLATES: &[ResolvedTemplate] = &[\n");
    for v in compiled {
        let id = v.canonical_id();
        let jinja = if v.jinja.is_some() {
            format!("Some({})", v.static_name("JINJA"))
        } else {
            "None".to_string()
        };
        let liquid = if v.liquid.is_some() {
            format!("Some({})", v.static_name("LIQUID"))
        } else {
            "None".to_string()
        };
        let archive_jinja = if v.jinja.is_some() {
            format!("Some({})", rust_string_literal(&v.archive_jinja_rel()))
        } else {
            "None".to_string()
        };
        let archive_liquid = if v.liquid.is_some() {
            format!("Some({})", rust_string_literal(&v.archive_liquid_rel()))
        } else {
            "None".to_string()
        };

        out.push_str("    ResolvedTemplate {\n");
        out.push_str("        meta: TemplateMeta {\n");
        out.push_str(&format!("            id: {},\n", rust_string_literal(&id)));
        out.push_str(&format!(
            "            name: {},\n",
            rust_string_literal(&v.name)
        ));
        out.push_str(&format!("            revision: {},\n", v.revision));
        out.push_str(&format!("            deprecated: {},\n", v.deprecated));
        out.push_str(&format!("            archive_path_jinja: {archive_jinja},\n"));
        out.push_str(&format!(
            "            archive_path_liquid: {archive_liquid},\n"
        ));
        out.push_str("        },\n");
        out.push_str(&format!("        jinja: {jinja},\n"));
        out.push_str(&format!("        liquid: {liquid},\n"));
        out.push_str("    },\n");
    }
    out.push_str("];\n\n");

    out.push_str(include_str!("registry_runtime.rs.inc"));

    fs::write("gen/template_registry.rs", out)
        .with_context(|| "Failed to write gen/template_registry.rs")?;
    Ok(())
}

fn write_manifest(compiled: &[CompiledTemplate]) -> Result<()> {
    let mut templates = Map::new();
    for v in compiled {
        let mut engines = Vec::new();
        if v.jinja.is_some() {
            engines.push(json!("jinja"));
        }
        if v.liquid.is_some() {
            engines.push(json!("liquid"));
        }
        templates.insert(
            v.name.clone(),
            json!({
                "revision": v.revision,
                "id": v.canonical_id(),
                "deprecated": v.deprecated,
                "engines": engines,
                "archive": {
                    "jinja": v.jinja.as_ref().map(|_| v.archive_jinja_rel()),
                    "liquid": v.liquid.as_ref().map(|_| v.archive_liquid_rel()),
                    "latest_jinja": v.jinja.as_ref().map(|_| format!("{}/latest.jinja", v.name)),
                    "latest_liquid": v.liquid.as_ref().map(|_| format!("{}/latest.tmpl", v.name)),
                },
                "curl_hint": format!(
                    "https://raw.githubusercontent.com/cohere-ai/melody/main/gen/templates/archive/{}/latest.jinja",
                    v.name
                ),
            }),
        );
    }

    let manifest = json!({
        "schema_version": 1,
        "id_format": "{name}@{revision}",
        "archive_layout": "{name}/{name}@{revision}.jinja",
        "latest_layout": "{name}/latest.jinja",
        "templates": templates,
    });

    let pretty = serde_json::to_string_pretty(&manifest)?;
    fs::write("gen/templates/manifest.json", pretty)
        .with_context(|| "Failed to write gen/templates/manifest.json")?;
    Ok(())
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(&args) {
        eprintln!("Error: {e:?}");
        std::process::exit(1);
    }
}

fn run(args: &Args) -> Result<()> {
    let input = fs::read_to_string(&args.registry)
        .with_context(|| format!("Failed to read registry: {}", args.registry))?;
    let registry: Registry = serde_yaml::from_str(&input)
        .with_context(|| format!("Failed to parse registry YAML: {}", args.registry))?;

    let compiled = compile_registry(&registry, args)?;
    enforce_revision_locks(&compiled)?;
    write_outputs(&compiled)?;
    write_rust_registry(&compiled)?;
    write_manifest(&compiled)?;

    println!(
        "Generated {} templates from {}",
        compiled.len(),
        args.registry
    );
    Ok(())
}
