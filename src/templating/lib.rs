use crate::errors::MelodyError;
use crate::templating::embedded_templates::{lookup_jinja, lookup_liquid};
use crate::templating::types::{
    CitationQuality, Document, Grounding, Message, ReasoningType, SafetyMode, Tool,
};
use crate::templating::util::{
    add_jinja_substitutions_cmd3, add_jinja_substitutions_cmd4, add_jinja_substitutions_common,
    docs_to_template, get_jinja_vars, get_minijinja_env, messages_to_template, tools_to_template,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

fn jinja_for_id(template_id: &str, family: &str) -> Result<&'static str, MelodyError> {
    let body = lookup_jinja(template_id).ok_or_else(|| {
        MelodyError::TemplateValidation(format!("unknown template id: {template_id}"))
    })?;
    // Resolve to the canonical name so pins like cmd4@1 still check cleanly.
    let name = template_id.split_once('@').map(|(n, _)| n).unwrap_or(template_id);
    let ok = match family {
        "cmd3" => name.starts_with("cmd3-"),
        "cmd4" => name == "cmd4" || name.starts_with("cmd4-"),
        "cmd5" => name == "cmd5" || name.starts_with("cmd5-"),
        _ => false,
    };
    if !ok {
        return Err(MelodyError::TemplateValidation(format!(
            "template id '{template_id}' is not valid for {family}"
        )));
    }
    Ok(body)
}

fn require_liquid(template_id: &str) -> &'static str {
    lookup_liquid(template_id)
        .unwrap_or_else(|| panic!("missing liquid template for id '{template_id}'"))
}

#[cfg(test)]
fn require_jinja(template_id: &str) -> &'static str {
    lookup_jinja(template_id)
        .unwrap_or_else(|| panic!("missing jinja template for id '{template_id}'"))
}

/// Options for cmd3 rendering.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RenderCmd3Options<'a> {
    /// Messages to include in the rendered prompt.
    pub messages: Vec<Message>,
    /// Optional template ID (`cmd3-reasoning`, `cmd3-legacy@1`, ...).
    pub template_id: Option<String>,
    /// Liquid template string to use for rendering.
    pub template: &'a str,
    /// Jinja template string. An empty string is treated as "caller did not
    /// provide a template" and the renderer falls back to `cmd3-legacy`.
    pub template_jinja: &'a str,
    /// Whether to use jinja template
    pub use_jinja: bool,
    /// Optional developer instruction to include in the prompt.
    pub dev_instruction: Option<String>,
    /// Documents to include for grounding.
    pub documents: Vec<Document>,
    /// Tools available to the model.
    pub available_tools: Vec<Tool>,
    /// Safety mode configuration.
    pub safety_mode: Option<SafetyMode>,
    /// Citation quality setting.
    pub citation_quality: Option<CitationQuality>,
    /// Reasoning/thinking mode configuration.
    pub reasoning_type: Option<ReasoningType>,
    /// Whether to skip the preamble section.
    pub skip_preamble: bool,
    /// Optional prefix for the response.
    pub response_prefix: Option<String>,
    /// Optional JSON schema for structured output.
    pub json_schema: Option<String>,
    /// Whether to enable JSON mode.
    pub json_mode: bool,
    /// Additional fields to substitute in the template.
    pub additional_template_fields: Map<String, Value>,
    /// Special tokens to escape in the output.
    pub escaped_special_tokens: BTreeMap<String, String>,
}

impl Default for RenderCmd3Options<'_> {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            template_id: None,
            // Liquid default stays on legacy for compatibility with liquid callers.
            template: require_liquid("cmd3-legacy"),
            template_jinja: "",
            use_jinja: false,
            dev_instruction: None,
            documents: Vec::new(),
            available_tools: Vec::new(),
            safety_mode: None,
            citation_quality: Some(CitationQuality::On),
            reasoning_type: None,
            skip_preamble: false,
            response_prefix: None,
            json_schema: None,
            json_mode: false,
            additional_template_fields: Map::new(),
            escaped_special_tokens: BTreeMap::new(),
        }
    }
}

/// Options for cmd4 rendering.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RenderCmd4Options<'a> {
    /// Messages to include in the rendered prompt.
    pub messages: Vec<Message>,
    /// Optional template ID (`cmd4`, `cmd4@1`, ...).
    pub template_id: Option<String>,
    /// Liquid template string to use for rendering.
    pub template: &'a str,
    /// Jinja template string. An empty string is treated as "caller did not
    /// provide a template" and the renderer falls back to `cmd4@1` when called via [`render_cmd4`].
    pub template_jinja: &'a str,
    /// Whether to use jinja template
    pub use_jinja: bool,
    /// Optional developer instruction to include in the prompt.
    pub dev_instruction: Option<String>,
    /// Optional platform instruction override.
    pub platform_instruction: Option<String>,
    /// Documents to include for grounding.
    pub documents: Vec<Document>,
    /// Tools available to the model.
    pub available_tools: Vec<Tool>,
    /// Grounding configuration.
    pub grounding: Option<Grounding>,
    /// Reasoning/thinking mode configuration.
    pub reasoning_type: Option<ReasoningType>,
    /// Optional prefix for the response.
    pub response_prefix: Option<String>,
    /// Optional JSON schema for structured output.
    pub json_schema: Option<String>,
    /// Whether to enable JSON mode.
    pub json_mode: bool,
    /// Additional fields to substitute in the template.
    pub additional_template_fields: Map<String, Value>,
    /// Special tokens to escape in the output.
    pub escaped_special_tokens: BTreeMap<String, String>,
    /// When true, tool call names are left unescaped for templates that apply
    /// XML attribute escaping (cmd5). Set internally by [`render_cmd5`]; do not
    /// set manually.
    #[serde(skip, default)]
    pub raw_tool_call_names: bool,
}

impl Default for RenderCmd4Options<'_> {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            template_id: None,
            // Liquid default stays on cmd4@1 for compatibility with liquid callers.
            template: require_liquid("cmd4@1"),
            template_jinja: "",
            use_jinja: false,
            dev_instruction: None,
            platform_instruction: None,
            documents: Vec::new(),
            available_tools: Vec::new(),
            grounding: Some(Grounding::Enabled),
            reasoning_type: None,
            response_prefix: None,
            json_schema: None,
            json_mode: false,
            additional_template_fields: Map::new(),
            escaped_special_tokens: BTreeMap::new(),
            raw_tool_call_names: false,
        }
    }
}

/// Options for cmd5 rendering.
///
/// CMD5 uses the same option fields as CMD4 (developer/platform instruction,
/// tools, documents, grounding, reasoning, json mode, etc.), so this is a
/// type alias rather than a separate struct.
pub type RenderCmd5Options<'a> = RenderCmd4Options<'a>;

fn validate_no_multipart(messages: &[Message]) -> Option<MelodyError> {
    for msg in messages {
        for content in &msg.content {
            if content.content_type == crate::templating::types::ContentType::Multipart {
                return Some(MelodyError::TemplateValidation(
                    "multipart content type is not supported for command 3".to_string(),
                ));
            }
        }
    }
    None
}

/// Renders a CMD3 format prompt from the given options.
///
/// # Errors
///
/// Returns a `MelodyError` if:
/// - JSON serialization of documents fails
/// - Template parsing fails
/// - Template rendering fails
#[allow(clippy::too_many_lines)]
pub fn render_cmd3(opts: &RenderCmd3Options) -> Result<String, MelodyError> {
    let mut template_tools = tools_to_template(&opts.available_tools)?;
    if let Some(err) = validate_no_multipart(&opts.messages) {
        return Err(err);
    }
    let mut messages = messages_to_template(
        &opts.messages,
        &opts.documents,
        &opts.escaped_special_tokens,
    )?;
    let mut docs = docs_to_template(&opts.documents, &opts.escaped_special_tokens)?;

    if opts.use_jinja {
        (messages, template_tools, docs) = get_jinja_vars(
            &messages,
            &opts.available_tools,
            &opts.documents,
            &opts.escaped_special_tokens,
            false,
        )?;
    }

    let mut substitutions = opts.additional_template_fields.clone();
    substitutions.insert(
        "preamble".to_string(),
        opts.dev_instruction
            .clone()
            .map_or(Value::Null, Value::String),
    );
    substitutions.insert("messages".to_string(), Value::Array(messages));
    substitutions.insert("documents".to_string(), Value::Array(docs));
    substitutions.insert(
        "available_tools".to_string(),
        Value::Array(template_tools.into_iter().map(Value::Object).collect()),
    );
    substitutions.insert(
        "citation_mode".to_string(),
        opts.citation_quality
            .as_ref()
            .map_or(Value::Null, |c| Value::String(c.as_str().to_string())),
    );
    substitutions.insert(
        "safety_mode".to_string(),
        opts.safety_mode
            .as_ref()
            .map_or(Value::Null, |s| Value::String(s.as_str().to_string())),
    );
    if opts.reasoning_type.is_some() {
        substitutions.insert(
            "reasoning_options".to_string(),
            Value::Object({
                let mut m = Map::new();
                m.insert(
                    "enabled".to_string(),
                    Value::Bool(matches!(opts.reasoning_type, Some(ReasoningType::Enabled))),
                );
                m
            }),
        );
    }
    substitutions.insert("skip_preamble".to_string(), Value::Bool(opts.skip_preamble));
    substitutions.insert(
        "skip_thinking".to_string(),
        Value::Bool(matches!(opts.reasoning_type, Some(ReasoningType::Disabled))),
    );
    substitutions.insert(
        "response_prefix".to_string(),
        opts.response_prefix
            .clone()
            .map_or(json!(""), Value::String),
    );
    substitutions.insert(
        "json_schema".to_string(),
        opts.json_schema.clone().map_or(Value::Null, Value::String),
    );
    substitutions.insert("json_mode".to_string(), Value::Bool(opts.json_mode));

    if opts.use_jinja {
        add_jinja_substitutions_common(
            &mut substitutions,
            opts.json_mode,
            &opts.json_schema,
            &opts.reasoning_type,
        );
        add_jinja_substitutions_cmd3(&mut substitutions, opts);

        let mut active_template = if opts.template_jinja.is_empty() {
            jinja_for_id("cmd3-legacy", "cmd3")?
        } else {
            opts.template_jinja
        };
        if let Some(template_id) = opts.template_id.as_ref() {
            active_template = jinja_for_id(template_id, "cmd3")?;
        }

        let template_name = "chat_template.jinja";
        let env = get_minijinja_env(template_name, active_template)?;
        let template = env.get_template(template_name)?;
        let template_str = template.render(&substitutions)?;

        Ok(template_str)
    } else {
        let parser = liquid::ParserBuilder::with_stdlib().build()?;
        let template = parser.parse(opts.template)?;

        Ok(template.render(&liquid::object!(&substitutions))?)
    }
}

/// Renders a CMD4 format prompt from the given options.
///
/// # Errors
///
/// Returns a `MelodyError` if:
/// - JSON serialization of documents fails
/// - Template parsing fails
/// - Template rendering fails
pub fn render_cmd4(opts: &RenderCmd4Options) -> Result<String, MelodyError> {
    let mut template_tools = tools_to_template(&opts.available_tools)?;
    let mut messages = messages_to_template(
        &opts.messages,
        &opts.documents,
        &opts.escaped_special_tokens,
    )?;
    let mut docs = docs_to_template(&opts.documents, &opts.escaped_special_tokens)?;

    if opts.use_jinja {
        (messages, template_tools, docs) = get_jinja_vars(
            &messages,
            &opts.available_tools,
            &opts.documents,
            &opts.escaped_special_tokens,
            opts.raw_tool_call_names,
        )?;
    }

    let mut substitutions = opts.additional_template_fields.clone();
    substitutions.insert(
        "developer_instruction".to_string(),
        opts.dev_instruction
            .clone()
            .map_or(Value::Null, Value::String),
    );
    substitutions.insert(
        "platform_instruction_override".to_string(),
        opts.platform_instruction
            .clone()
            .map_or(Value::Null, Value::String),
    );
    substitutions.insert("messages".to_string(), Value::Array(messages));
    substitutions.insert("documents".to_string(), Value::Array(docs));
    substitutions.insert(
        "available_tools".to_string(),
        Value::Array(template_tools.into_iter().map(Value::Object).collect()),
    );
    substitutions.insert(
        "grounding".to_string(),
        opts.grounding
            .as_ref()
            .map_or(Value::Null, |g| Value::String(g.as_str().to_string())),
    );
    substitutions.insert(
        "response_prefix".to_string(),
        opts.response_prefix
            .clone()
            .map_or(json!(""), Value::String),
    );
    substitutions.insert(
        "json_schema".to_string(),
        opts.json_schema.clone().map_or(Value::Null, Value::String),
    );
    substitutions.insert("json_mode".to_string(), Value::Bool(opts.json_mode));

    if opts.use_jinja {
        add_jinja_substitutions_common(
            &mut substitutions,
            opts.json_mode,
            &opts.json_schema,
            &opts.reasoning_type,
        );
        add_jinja_substitutions_cmd4(&mut substitutions, opts);

        let mut active_template = if opts.template_jinja.is_empty() {
            jinja_for_id("cmd4@1", "cmd4")?
        } else {
            opts.template_jinja
        };
        if let Some(template_id) = opts.template_id.as_ref() {
            active_template = jinja_for_id(template_id, "cmd4")?;
        }

        let template_name = "chat_template.jinja";
        let env = get_minijinja_env(template_name, active_template)?;
        let template = env.get_template(template_name)?;
        let template_str = template.render(&substitutions)?;

        Ok(template_str)
    } else {
        let parser = liquid::ParserBuilder::with_stdlib().build()?;
        let template = parser.parse(opts.template)?;

        Ok(template.render(&liquid::object!(&substitutions))?)
    }
}

/// Renders a CMD5 format prompt from the given options.
///
/// CMD5 shares its option schema with CMD4 (`RenderCmd5Options` is a type alias
/// for `RenderCmd4Options`). This function differs from `render_cmd4` only in
/// that it defaults to the CMD5 jinja template and resolves `template_id` values
/// against built-in template ids. Jinja rendering is always used (CMD5 has no
/// liquid variant).
///
/// Template selection priority (mirrors `render_cmd4`):
/// 1. `opts.template_id` if set.
/// 2. `opts.template_jinja` if non-empty (caller supplied a custom template).
/// 3. `cmd5` otherwise.
///
/// # Errors
///
/// Returns a `MelodyError` if:
/// - JSON serialization of documents fails
/// - Template parsing fails
/// - Template rendering fails
pub fn render_cmd5<'a>(opts: &RenderCmd5Options<'a>) -> Result<String, MelodyError> {
    // CMD5 is jinja-only. Reject explicit liquid-template requests regardless of
    // entry point (Rust API, Python, or FFI after option conversion). An empty
    // FFI `template` pointer leaves the CMD4 liquid default on the struct; that
    // unused default is not treated as a caller-provided liquid template. A
    // leftover liquid `template` field is likewise ignored when `template_id` or
    // non-empty `template_jinja` selects a jinja path.
    let cmd4_v1_liquid = require_liquid("cmd4@1");
    if !opts.use_jinja
        && opts.template != cmd4_v1_liquid
        && opts.template_id.is_none()
        && opts.template_jinja.is_empty()
    {
        return Err(MelodyError::TemplateValidation(
            "CMD5 does not support liquid templates; use template_jinja or template_id instead"
                .to_string(),
        ));
    }

    let mut active_opts: RenderCmd5Options<'a> = opts.clone();
    // Honor caller `template_jinja` even when `use_jinja` is false.
    active_opts.use_jinja = true;
    // cmd5 templates XML-escape tool names via `xml_attr`; skip JSON escaping
    // in the jinja message prep path (cmd3/cmd4 embed names in JSON instead).
    active_opts.raw_tool_call_names = true;

    if let Some(template_id) = opts.template_id.as_ref() {
        active_opts.template_jinja = jinja_for_id(template_id, "cmd5")?;
        active_opts.template_id = None;
    } else if active_opts.template_jinja.is_empty() {
        // Pin the cmd5 default before delegating to `render_cmd4`, whose own
        // empty-string fallback would otherwise pick cmd4@1.
        active_opts.template_jinja = jinja_for_id("cmd5", "cmd5")?;
    }

    render_cmd4(&active_opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::Value;
    use serde_path_to_error::deserialize;
    use std::fs;
    use std::path::Path;

    fn templating_test_dir(version: &str) -> std::path::PathBuf {
        let cur_file = file!();
        let cur_dir = Path::new(cur_file)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        cur_dir.join("tests/templating").join(version)
    }

    /// Iterates the fixtures in `tests/templating/<version>`. Each entry yields
    /// the test name, parsed `input.json`, the contents of `output.txt` (empty
    /// string if the file does not yet exist), and the path to `output.txt`.
    ///
    /// Test cases without an `output.txt` will fail their assertions, signaling
    /// that the fixture needs to be generated. The `regenerate_*_fixtures`
    /// helpers below reuse this function and ignore the (possibly empty)
    /// output contents, writing fresh output to `output_path`.
    fn read_test_cases(version: &str) -> Vec<(String, Value, String, std::path::PathBuf)> {
        let mut cases = vec![];
        let test_dir = templating_test_dir(version);
        if !test_dir.exists() {
            panic!("Test directory {:?} does not exist.", test_dir);
        }
        for entry in fs::read_dir(&test_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                let input_path = path.join("input.json");
                let output_path = path.join("output.txt");
                if input_path.exists() {
                    let input = fs::read_to_string(&input_path).unwrap();
                    let input_json: Value = serde_json::from_str(&input).unwrap();
                    let output = if output_path.exists() {
                        fs::read_to_string(&output_path).unwrap()
                    } else {
                        String::new()
                    };
                    let test_name = path.file_name().unwrap().to_string_lossy().to_string();
                    cases.push((test_name, input_json, output, output_path));
                }
            }
        }
        cases
    }

    #[test]
    fn test_render_cmd3_from_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("cmd3") {
            println!("Running cmd3 test case: {}", test_name);
            let opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd3v3_from_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("cmd3_v3") {
            println!("Running cmd3v3 test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            if test_name != "template_provided" {
                opts.template = require_liquid("cmd3-default-thinking");
            }
            if opts.reasoning_type.is_none() || opts.reasoning_type == Some(ReasoningType::Unknown)
            {
                // Default for cmd3v3 on platform is reasoning is enabled
                opts.reasoning_type = Some(ReasoningType::Enabled);
            }
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd3_v1_jinja_from_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("jinja/cmd3_v1") {
            println!("Running cmd3 v1 jinja test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            opts.use_jinja = true;
            opts.template_jinja = require_jinja("cmd3-legacy-hf");
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd3_v2_jinja_from_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("jinja/cmd3_v2") {
            println!("Running cmd3 v2 jinja test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            opts.use_jinja = true;
            opts.template_jinja = require_jinja("cmd3-reasoning");
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd3_v3_jinja_from_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("jinja/cmd3_v3") {
            println!("Running cmd3 v3 jinja test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            opts.use_jinja = true;
            opts.template_jinja = require_jinja("cmd3-default-thinking");
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd4_v2_jinja_from_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("jinja/cmd4_v2") {
            println!("Running cmd4 v2 jinja test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            opts.use_jinja = true;
            opts.template_jinja = require_jinja("cmd4");
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd3_jinja_from_liquid_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("cmd3") {
            println!("Running cmd3 jinja liquid test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            opts.use_jinja = true;
            // Liquid fixtures match cmd3-legacy jinja output.
            if test_name != "template_provided" {
                opts.template_id = Some("cmd3-legacy".to_string());
            }
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd3v3_jinja_from_liquid_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("cmd3_v3") {
            println!("Running cmd3v3 jinja liquid test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            opts.use_jinja = true;
            if test_name != "template_provided" {
                opts.template_jinja = require_jinja("cmd3-default-thinking");
            }
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd4_from_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("cmd4") {
            println!("Running cmd4 test case: {}", test_name);
            let opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd4_jinja_from_liquid_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("cmd4") {
            println!("Running cmd4 jinja liquid test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            opts.use_jinja = true;
            // Liquid fixtures match cmd4@1 jinja output.
            if test_name != "template_provided" {
                opts.template_id = Some("cmd4@1".to_string());
            }
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd4_v2_jinja_from_liquid_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("cmd4_v2") {
            println!("Running cmd4 v2 jinja liquid test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            opts.use_jinja = true;
            if test_name != "template_provided" {
                opts.template_jinja = require_jinja("cmd4");
            }
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd4_hf_jinja_from_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("jinja/cmd4_hf") {
            println!("Running cmd4 hf jinja test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            opts.use_jinja = true;
            opts.template_jinja = require_jinja("cmd4-hf");
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd4_hf_jinja_from_liquid_dir() {
        for (test_name, input_json, expected, _) in read_test_cases("cmd4_hf") {
            println!("Running cmd4 hf jinja liquid test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            opts.use_jinja = true;
            if test_name != "template_provided" {
                opts.template_jinja = require_jinja("cmd4-hf");
            }
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    fn render_cmd5_from_input(input_json: &Value) -> String {
        let opts = deserialize::<_, RenderCmd5Options>(input_json).unwrap();
        render_cmd5(&opts).unwrap()
    }

    #[test]
    fn test_render_cmd5_jinja_from_dir() {
        let mut ran_any = false;
        for (test_name, input_json, expected, _) in read_test_cases("jinja/cmd5") {
            println!("Running cmd5 jinja test case: {}", test_name);
            let rendered = render_cmd5_from_input(&input_json);
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
            ran_any = true;
        }
        assert!(
            ran_any,
            "no cmd5 jinja test fixtures were found in tests/templating/jinja/cmd5"
        );
    }

    #[test]
    fn test_render_cmd5_no_escape_jinja_from_dir() {
        let mut ran_any = false;
        for (test_name, input_json, expected, _) in read_test_cases("jinja/cmd5_no_escape") {
            println!("Running cmd5-no-escape jinja test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd5Options>(&input_json).unwrap();
            opts.template_id = Some("cmd5-no-escape".to_string());
            let rendered = render_cmd5(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
            ran_any = true;
        }
        assert!(
            ran_any,
            "no cmd5-no-escape jinja test fixtures were found in tests/templating/jinja/cmd5_no_escape"
        );
    }

    #[test]
    fn test_render_cmd5_template_id_no_escape() {
        let opts = RenderCmd5Options {
            template_id: Some("cmd5-no-escape".to_string()),
            ..RenderCmd5Options::default()
        };
        let rendered = render_cmd5(&opts).unwrap();
        assert!(
            rendered.contains("CHATBOT"),
            "template_id=cmd5-no-escape should render via jinja"
        );
    }

    #[test]
    fn test_render_cmd5_strict_jinja_from_dir() {
        let mut ran_any = false;
        for (test_name, input_json, expected, _) in read_test_cases("jinja/cmd5_strict") {
            println!("Running cmd5-strict jinja test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd5Options>(&input_json).unwrap();
            opts.template_id = Some("cmd5-strict".to_string());
            let rendered = render_cmd5(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
            ran_any = true;
        }
        assert!(
            ran_any,
            "no cmd5-strict jinja test fixtures were found in tests/templating/jinja/cmd5_strict"
        );
    }

    #[test]
    fn test_render_cmd5_template_id_strict() {
        let opts = RenderCmd5Options {
            template_id: Some("cmd5-strict".to_string()),
            ..RenderCmd5Options::default()
        };
        let rendered = render_cmd5(&opts).unwrap();
        assert!(
            rendered.contains("CHATBOT"),
            "template_id=cmd5-strict should render via jinja"
        );
    }

    #[test]
    fn test_render_cmd5_preserves_custom_template_jinja() {
        // Sentinel template that does not depend on any of the cmd5 substitutions so
        // we can prove the caller-supplied `template_jinja` is what actually got rendered.
        let custom_template = "CUSTOM_CMD5_TEMPLATE_OUTPUT";
        let opts = RenderCmd5Options {
            use_jinja: true,
            template_jinja: custom_template,
            ..RenderCmd5Options::default()
        };
        let rendered = render_cmd5(&opts).unwrap();
        assert_eq!(
            rendered, custom_template,
            "render_cmd5 must use the caller-supplied template_jinja instead of CMD5_JINJA_TEMPLATE"
        );
    }

    #[test]
    fn test_render_cmd5_preserves_custom_template_jinja_without_use_jinja_flag() {
        let custom_template = "CUSTOM_CMD5_TEMPLATE_OUTPUT";
        let opts = RenderCmd5Options {
            use_jinja: false,
            template_jinja: custom_template,
            ..RenderCmd5Options::default()
        };
        let rendered = render_cmd5(&opts).unwrap();
        assert_eq!(rendered, custom_template);
    }

    #[test]
    fn test_render_cmd5_rejects_liquid_template() {
        let opts = RenderCmd5Options {
            use_jinja: false,
            template: "{% for msg in messages %}{{ msg }}{% endfor %}",
            ..RenderCmd5Options::default()
        };
        let err = render_cmd5(&opts).unwrap_err();
        assert!(
            err.to_string()
                .contains("does not support liquid templates"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_render_cmd5_template_id_ignores_leftover_liquid_template() {
        let opts = RenderCmd5Options {
            use_jinja: false,
            template_id: Some("cmd5".to_string()),
            template: "{% for msg in messages %}{{ msg }}{% endfor %}",
            ..RenderCmd5Options::default()
        };
        let rendered = render_cmd5(&opts).unwrap();
        assert!(
            rendered.contains("CHATBOT"),
            "template_id=cmd5 should render via jinja, ignoring leftover liquid template"
        );
    }

    #[test]
    fn test_render_cmd5_template_jinja_ignores_leftover_liquid_template() {
        let custom_template = "CUSTOM_CMD5_TEMPLATE_OUTPUT";
        let opts = RenderCmd5Options {
            use_jinja: false,
            template: "{% liquid %}",
            template_jinja: custom_template,
            ..RenderCmd5Options::default()
        };
        let rendered = render_cmd5(&opts).unwrap();
        assert_eq!(rendered, custom_template);
    }

    /// Helper test that rewrites every `output.txt` under `tests/templating/jinja/cmd5_no_escape`
    /// from the current cmd5-no-escape template output.
    #[test]
    #[ignore = "fixture regenerator; run on demand via `--ignored`"]
    fn regenerate_cmd5_no_escape_jinja_fixtures() {
        let cases = read_test_cases("jinja/cmd5_no_escape");
        assert!(
            !cases.is_empty(),
            "no cmd5-no-escape jinja input fixtures were found in tests/templating/jinja/cmd5_no_escape"
        );
        for (test_name, input_json, _, output_path) in cases {
            let mut opts = deserialize::<_, RenderCmd5Options>(&input_json).unwrap();
            opts.template_id = Some("cmd5-no-escape".to_string());
            let rendered = render_cmd5(&opts).unwrap();
            let bytes_written = rendered.len();
            fs::write(&output_path, &rendered)
                .unwrap_or_else(|e| panic!("failed to write {output_path:?} for {test_name}: {e}"));
            println!("Wrote fixture: {test_name} ({bytes_written} bytes)");
        }
    }

    /// Helper test that rewrites every `output.txt` under `tests/templating/jinja/cmd5`
    /// from the current cmd5 template output. It is `#[ignore]`d so it never runs as
    /// part of `cargo test`; opt in by running:
    ///
    /// ```bash
    /// cargo test -p melody-parsing regenerate_cmd5_jinja_fixtures -- --ignored --nocapture
    /// ```
    ///
    /// Use this after intentional changes to `cmd5.jinja` or after adding new
    /// `input.json` cases to bootstrap the matching `output.txt`.
    #[test]
    #[ignore = "fixture regenerator; run on demand via `--ignored`"]
    fn regenerate_cmd5_jinja_fixtures() {
        let cases = read_test_cases("jinja/cmd5");
        assert!(
            !cases.is_empty(),
            "no cmd5 jinja input fixtures were found in tests/templating/jinja/cmd5"
        );
        for (test_name, input_json, _, output_path) in cases {
            let rendered = render_cmd5_from_input(&input_json);
            let bytes_written = rendered.len();
            fs::write(&output_path, &rendered)
                .unwrap_or_else(|e| panic!("failed to write {output_path:?} for {test_name}: {e}"));
            println!("Wrote fixture: {test_name} ({bytes_written} bytes)");
        }
    }

    /// Helper test that rewrites every `output.txt` under `tests/templating/jinja/cmd5_strict`
    /// from the current cmd5-strict template output.
    #[test]
    #[ignore = "fixture regenerator; run on demand via `--ignored`"]
    fn regenerate_cmd5_strict_jinja_fixtures() {
        let cases = read_test_cases("jinja/cmd5_strict");
        assert!(
            !cases.is_empty(),
            "no cmd5-strict jinja input fixtures were found in tests/templating/jinja/cmd5_strict"
        );
        for (test_name, input_json, _, output_path) in cases {
            let mut opts = deserialize::<_, RenderCmd5Options>(&input_json).unwrap();
            opts.template_id = Some("cmd5-strict".to_string());
            let rendered = render_cmd5(&opts).unwrap();
            let bytes_written = rendered.len();
            fs::write(&output_path, &rendered)
                .unwrap_or_else(|e| panic!("failed to write {output_path:?} for {test_name}: {e}"));
            println!("Wrote fixture: {test_name} ({bytes_written} bytes)");
        }
    }

    #[test]
    fn test_lookup_embedded_templates() {
        use crate::templating::embedded_templates::{lookup_jinja, lookup_liquid};

        assert!(lookup_jinja("cmd4@2").is_some());
        assert!(lookup_jinja("cmd4").is_some());
        assert_eq!(lookup_jinja("cmd4"), lookup_jinja("cmd4@2"));
        assert!(lookup_jinja("cmd4@1").is_some());
        assert_ne!(lookup_jinja("cmd4@1"), lookup_jinja("cmd4@2"));
        assert!(lookup_liquid("cmd4@1").is_some());
        assert!(lookup_liquid("cmd4").is_none()); // current cmd4@2 is jinja-only
        assert!(lookup_jinja("cmd5").is_some());
        assert!(lookup_jinja("cmd5-no-escape").is_some());
        assert!(lookup_jinja("cmd4-does-not-exist").is_none());
        assert!(lookup_jinja("cmd4-v2").is_none());
        assert!(lookup_jinja("cmd4-classic").is_none());
        assert!(lookup_liquid("cmd5").is_none());
    }

    #[test]
    fn test_jinja_for_id_rejects_cross_family() {
        assert!(jinja_for_id("cmd5", "cmd3").is_err());
        assert!(jinja_for_id("cmd4", "cmd3").is_err());
        assert!(jinja_for_id("cmd5", "cmd4").is_err());
        assert!(jinja_for_id("cmd3-reasoning", "cmd4").is_err());
        assert!(jinja_for_id("cmd4", "cmd5").is_err());
        assert!(jinja_for_id("cmd3-legacy", "cmd5").is_err());

        assert!(jinja_for_id("cmd3-reasoning", "cmd3").is_ok());
        assert!(jinja_for_id("cmd4@1", "cmd4").is_ok());
        assert!(jinja_for_id("cmd4", "cmd4").is_ok());
        assert!(jinja_for_id("cmd5", "cmd5").is_ok());
        assert!(jinja_for_id("cmd5-no-escape", "cmd5").is_ok());
    }

    #[test]
    fn test_render_rejects_cross_family_template_id() {
        let err = render_cmd4(&RenderCmd4Options {
            template_id: Some("cmd5".to_string()),
            use_jinja: true,
            ..Default::default()
        })
        .unwrap_err();
        assert!(
            err.to_string().contains("not valid for cmd4"),
            "unexpected error: {err}"
        );

        let err = render_cmd5(&RenderCmd5Options {
            template_id: Some("cmd4".to_string()),
            ..Default::default()
        })
        .unwrap_err();
        assert!(
            err.to_string().contains("not valid for cmd5"),
            "unexpected error: {err}"
        );

        let err = render_cmd3(&RenderCmd3Options {
            template_id: Some("cmd4@1".to_string()),
            use_jinja: true,
            ..Default::default()
        })
        .unwrap_err();
        assert!(
            err.to_string().contains("not valid for cmd3"),
            "unexpected error: {err}"
        );
    }
}
