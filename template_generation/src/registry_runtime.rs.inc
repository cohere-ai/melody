/// List registered templates.
pub fn list_templates() -> Vec<TemplateMeta> {
    TEMPLATES.iter().map(|t| t.meta).collect()
}

fn latest_for(name: &str) -> Option<&'static ResolvedTemplate> {
    TEMPLATES
        .iter()
        .filter(|t| t.meta.name == name)
        .max_by_key(|t| t.meta.revision)
}

fn find_exact(name: &str, revision: u32) -> Option<&'static ResolvedTemplate> {
    TEMPLATES
        .iter()
        .find(|t| t.meta.name == name && t.meta.revision == revision)
}

/// Resolve a template id.
///
/// Accepted forms:
/// - `{name}@{revision}` — exact pin (matches archive filename without extension)
/// - `{name}` — latest revision for that name
pub fn resolve_template_id(id: &str) -> Result<&'static ResolvedTemplate, String> {
    match id.rsplit_once('@') {
        Some((name, rev)) => {
            if name.is_empty() {
                return Err(format!("unknown template id: {id}"));
            }
            let revision: u32 = rev
                .parse()
                .map_err(|_| format!("invalid template revision in id: {id}"))?;
            find_exact(name, revision).ok_or_else(|| format!("unknown template id: {id}"))
        }
        None => latest_for(id).ok_or_else(|| format!("unknown template id: {id}")),
    }
}

/// Jinja body for a named template (latest revision).
pub fn template_jinja(name: &str) -> Result<&'static str, String> {
    let resolved =
        latest_for(name).ok_or_else(|| format!("unknown template id: {name}"))?;
    resolved
        .jinja
        .ok_or_else(|| format!("template '{name}' has no jinja engine"))
}

/// Liquid body for a named template (latest revision), if present.
pub fn template_liquid(name: &str) -> Result<&'static str, String> {
    let resolved =
        latest_for(name).ok_or_else(|| format!("unknown template id: {name}"))?;
    resolved
        .liquid
        .ok_or_else(|| format!("template '{name}' has no liquid engine"))
}
