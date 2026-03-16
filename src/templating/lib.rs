use crate::errors::MelodyError;
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
use std::str::FromStr;

/// Options for cmd3 rendering.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RenderCmd3Options<'a> {
    /// Messages to include in the rendered prompt.
    pub messages: Vec<Message>,
    /// Optional template ID to use instead of template string
    pub template_id: Option<String>,
    /// Template string to use for rendering.
    pub template: &'a str,
    /// Jinja template string
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
// for now always set the template to cmd3v1.
static CMD3V1_TEMPLATE: &str = include_str!("../../gen/templates/liquid/cmd3-v1.tmpl");
static CMD3V1_JINJA_TEMPLATE: &str = include_str!("../../gen/templates/jinja/cmd3-v1.jinja");
static CMD3V2_JINJA_TEMPLATE: &str = include_str!("../../gen/templates/jinja/cmd3-v2.jinja");
static CMD3V3_JINJA_TEMPLATE: &str = include_str!("../../gen/templates/jinja/cmd3-v3.jinja");

impl Default for RenderCmd3Options<'_> {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            template_id: None,
            template: CMD3V1_TEMPLATE,
            template_jinja: CMD3V1_JINJA_TEMPLATE,
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
    /// Optional template ID to use instead of template string
    pub template_id: Option<String>,
    /// Template string to use for rendering.
    pub template: &'a str,
    /// Jinja template string
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

static CMD4V1_TEMPLATE: &str = include_str!("../../gen/templates/liquid/cmd4-v1.tmpl");
static CMD4V1_JINJA_TEMPLATE: &str = include_str!("../../gen/templates/jinja/cmd4-v1.jinja");
impl Default for RenderCmd4Options<'_> {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            template_id: None,
            template: CMD4V1_TEMPLATE,
            template_jinja: CMD4V1_JINJA_TEMPLATE,
            use_jinja: false,
            dev_instruction: None,
            platform_instruction: None,
            documents: Vec::new(),
            available_tools: Vec::new(),
            grounding: Some(Grounding::Enabled),
            response_prefix: None,
            json_schema: None,
            json_mode: false,
            additional_template_fields: Map::new(),
            escaped_special_tokens: BTreeMap::new(),
        }
    }
}

enum CMD3JinjaTemplates {
    CMD3V1,
    CMD3V2,
    CMD3V3,
}

impl FromStr for CMD3JinjaTemplates {
    type Err = MelodyError;

    fn from_str(o: &str) -> Result<Self, Self::Err> {
        match o {
            "cmd3-v1" => Ok(Self::CMD3V1),
            "cmd3-v2" => Ok(Self::CMD3V2),
            "cmd3-v3" => Ok(Self::CMD3V3),
            _ => Err(MelodyError::TemplateValidation(format!(
                "unknown template id: {o}"
            ))),
        }
    }
}

impl CMD3JinjaTemplates {
    fn get_template(&self) -> &str {
        match *self {
            CMD3JinjaTemplates::CMD3V1 => CMD3V1_JINJA_TEMPLATE,
            CMD3JinjaTemplates::CMD3V2 => CMD3V2_JINJA_TEMPLATE,
            CMD3JinjaTemplates::CMD3V3 => CMD3V3_JINJA_TEMPLATE,
        }
    }
}

enum CMD4JinjaTemplates {
    CMD4V1,
}

impl CMD4JinjaTemplates {
    fn get_template(&self) -> &str {
        match *self {
            CMD4JinjaTemplates::CMD4V1 => CMD4V1_JINJA_TEMPLATE,
        }
    }
}

impl FromStr for CMD4JinjaTemplates {
    type Err = MelodyError;

    fn from_str(o: &str) -> Result<Self, Self::Err> {
        match o {
            "cmd4-v1" => Ok(Self::CMD4V1),
            _ => Err(MelodyError::TemplateValidation(format!(
                "unknown template id: {o}"
            ))),
        }
    }
}

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
pub fn render_cmd3(opts: &RenderCmd3Options) -> Result<String, MelodyError> {
    let mut template_tools = tools_to_template(&opts.available_tools)?;
    if let Some(err) = validate_no_multipart(&opts.messages) {
        return Err(err);
    }
    let mut messages = messages_to_template(
        &opts.messages,
        !opts.documents.is_empty(),
        &opts.escaped_special_tokens,
    )?;
    let mut docs = docs_to_template(&opts.documents, &opts.escaped_special_tokens)?;

    if opts.use_jinja {
        (messages, template_tools, docs) = get_jinja_vars(
            &messages,
            &opts.available_tools,
            &opts.documents,
            &opts.escaped_special_tokens,
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
        add_jinja_substitutions_common(&mut substitutions, opts.json_mode, &opts.json_schema);
        add_jinja_substitutions_cmd3(&mut substitutions, opts);

        let mut active_template = opts.template_jinja;
        let template_enum: CMD3JinjaTemplates;
        if let Some(template_id) = opts.template_id.as_ref() {
            template_enum = CMD3JinjaTemplates::from_str(template_id)?;
            active_template = template_enum.get_template();
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
        !opts.documents.is_empty(),
        &opts.escaped_special_tokens,
    )?;
    let mut docs = docs_to_template(&opts.documents, &opts.escaped_special_tokens)?;

    if opts.use_jinja {
        (messages, template_tools, docs) = get_jinja_vars(
            &messages,
            &opts.available_tools,
            &opts.documents,
            &opts.escaped_special_tokens,
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
        add_jinja_substitutions_common(&mut substitutions, opts.json_mode, &opts.json_schema);
        add_jinja_substitutions_cmd4(&mut substitutions, opts);

        let mut active_template = opts.template_jinja;
        let template_enum: CMD4JinjaTemplates;
        if let Some(template_id) = opts.template_id.as_ref() {
            template_enum = CMD4JinjaTemplates::from_str(template_id)?;
            active_template = template_enum.get_template();
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::Value;
    use serde_path_to_error::deserialize;
    use std::fs;
    use std::path::Path;

    fn read_test_cases(version: &str) -> Vec<(String, Value, String)> {
        let mut cases = vec![];
        let cur_file = file!();
        let cur_dir = Path::new(cur_file)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let test_dir = cur_dir.join("tests/templating").join(version);
        if !test_dir.exists() {
            panic!("Test directory {:?} does not exist.", test_dir);
        }
        for entry in fs::read_dir(&test_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                let input_path = path.join("input.json");
                let output_path = path.join("output.txt");
                if input_path.exists() && output_path.exists() {
                    let input = fs::read_to_string(&input_path).unwrap();
                    let input_json: Value = serde_json::from_str(&input).unwrap();
                    let output = fs::read_to_string(&output_path).unwrap();
                    let test_name = path.file_name().unwrap().to_string_lossy().to_string();
                    cases.push((test_name, input_json, output));
                }
            }
        }
        cases
    }

    #[test]
    fn test_render_cmd3_from_dir() {
        for (test_name, input_json, expected) in read_test_cases("cmd3") {
            println!("Running cmd3 test case: {}", test_name);
            let opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd3_jinja_from_dir() {
        for (test_name, input_json, expected) in read_test_cases("jinja/cmd3_v1") {
            println!("Running cmd3 jinja test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            opts.use_jinja = true;
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd3_jinja_from_liquid_dir() {
        for (test_name, input_json, expected) in read_test_cases("cmd3") {
            println!("Running cmd3 jinja liquid test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            opts.use_jinja = true;
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd3v3_jinja_from_liquid_dir() {
        for (test_name, input_json, expected) in read_test_cases("cmd3_v3") {
            println!("Running cmd3 jinja liquid test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            opts.use_jinja = true;
            if test_name != "template_provided" {
                opts.template_jinja = CMD3V3_JINJA_TEMPLATE;
            }
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd4_from_dir() {
        for (test_name, input_json, expected) in read_test_cases("cmd4") {
            println!("Running cmd4 test case: {}", test_name);
            let opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }

    #[test]
    fn test_render_cmd4_jinja_from_liquid_dir() {
        for (test_name, input_json, expected) in read_test_cases("cmd4") {
            println!("Running cmd4 jinja liquid test case: {}", test_name);
            let mut opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            opts.use_jinja = true;
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }
}
