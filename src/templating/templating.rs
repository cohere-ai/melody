use std::collections::BTreeMap;
use std::error::Error;
use serde_json::{Map, Value};
use crate::templating::types::*;
use crate::templating::util::*;


/// Options for cmd3 rendering.
pub struct RenderCmd3Options {
    pub template: String,
    pub dev_instruction: Option<String>,
    pub documents: Vec<Document>,
    pub available_tools: Vec<Tool>,
    pub safety_mode: Option<SafetyMode>,
    pub citation_quality: Option<CitationQuality>,
    pub reasoning_type: Option<ReasoningType>,
    pub skip_preamble: bool,
    pub response_prefix: Option<String>,
    pub json_schema: Option<String>,
    pub json_mode: bool,
    pub additional_template_fields: BTreeMap<String, Value>,
    pub escaped_special_tokens: BTreeMap<String, String>,
}

/// Options for cmd4 rendering.
pub struct RenderCmd4Options {
    pub template: String,
    pub dev_instruction: Option<String>,
    pub platform_instruction: Option<String>,
    pub documents: Vec<Document>,
    pub available_tools: Vec<Tool>,
    pub grounding: Option<Grounding>,
    pub response_prefix: Option<String>,
    pub json_schema: Option<String>,
    pub json_mode: bool,
    pub additional_template_fields: BTreeMap<String, Value>,
    pub escaped_special_tokens: BTreeMap<String, String>,
}

pub fn render_cmd3(messages: &[Message], opts: &RenderCmd3Options) -> Result<String, Box<dyn Error>> {
    let template_tools = tools_to_template(&opts.available_tools)?;
    let messages = messages_to_template(
        messages,
        !opts.documents.is_empty(),
        &opts.escaped_special_tokens,
    )?;
    let docs: Vec<String> = opts
        .documents
        .iter()
        .map(|d| escape_special_tokens(d, &opts.escaped_special_tokens))
        .collect();

    let mut substitutions = opts.additional_template_fields.clone();
    substitutions.insert(
        "preamble".to_string(),
        opts.dev_instruction
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    substitutions.insert("messages".to_string(), Value::Array(messages));
    substitutions.insert(
        "documents".to_string(),
        Value::Array(docs.into_iter().map(Value::String).collect()),
    );
    substitutions.insert("available_tools".to_string(), Value::Array(template_tools.into_iter().map(Value::Object).collect()));
    substitutions.insert(
        "citation_mode".to_string(),
        opts.citation_quality
            .as_ref()
            .map(|c| Value::String(c.as_str().to_string()))
            .unwrap_or(Value::Null),
    );
    substitutions.insert(
        "safety_mode".to_string(),
        opts.safety_mode
            .as_ref()
            .map(|s| Value::String(s.as_str().to_string()))
            .unwrap_or(Value::Null),
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
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    substitutions.insert(
        "json_schema".to_string(),
        opts.json_schema
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    substitutions.insert("json_mode".to_string(), Value::Bool(opts.json_mode));

    let template = liquid::ParserBuilder::with_stdlib()
        .build().unwrap()
        .parse(&opts.template)
        .unwrap();

    Ok(template.render(&liquid::object!(&substitutions)).unwrap())
}

pub fn render_cmd4(messages: &[Message], opts: &RenderCmd4Options) -> Result<String, Box<dyn Error>> {
    let template_tools = tools_to_template(&opts.available_tools)?;
    let messages = messages_to_template(
        messages,
        !opts.documents.is_empty(),
        &opts.escaped_special_tokens,
    )?;
    let docs: Vec<String> = opts
        .documents
        .iter()
        .map(|d| escape_special_tokens(d, &opts.escaped_special_tokens))
        .collect();

    let mut substitutions = opts.additional_template_fields.clone();
    substitutions.insert(
        "developer_instruction".to_string(),
        opts.dev_instruction
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    substitutions.insert(
        "platform_instruction_override".to_string(),
        opts.platform_instruction
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    substitutions.insert("messages".to_string(), Value::Array(messages));
    substitutions.insert(
        "documents".to_string(),
        Value::Array(docs.into_iter().map(Value::String).collect()),
    );
    substitutions.insert("available_tools".to_string(), Value::Array(template_tools.into_iter().map(Value::Object).collect()));
    substitutions.insert(
        "grounding".to_string(),
        opts.grounding
            .as_ref()
            .map(|g| Value::String(g.as_str().to_string()))
            .unwrap_or(Value::Null),
    );
    substitutions.insert(
        "response_prefix".to_string(),
        opts.response_prefix
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    substitutions.insert(
        "json_schema".to_string(),
        opts.json_schema
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    substitutions.insert("json_mode".to_string(), Value::Bool(opts.json_mode));

    let template = liquid::ParserBuilder::with_stdlib()
        .build().unwrap()
        .parse(&opts.template)
        .unwrap();

    Ok(template.render(&liquid::object!(&substitutions)).unwrap())
}
