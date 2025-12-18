use crate::templating::types::*;
use crate::templating::util::*;
use serde_path_to_error::deserialize;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::error::Error;


/// Options for cmd3 rendering.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RenderCmd3Options {
    pub messages: Vec<Message>,
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

impl Default for RenderCmd3Options {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            template: String::new(),
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
            additional_template_fields: BTreeMap::new(),
            escaped_special_tokens: BTreeMap::new(),
        }
    }
}

/// Options for cmd4 rendering.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RenderCmd4Options {
    pub messages: Vec<Message>,
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

impl Default for RenderCmd4Options {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            template: String::new(),
            dev_instruction: None,
            platform_instruction: None,
            documents: Vec::new(),
            available_tools: Vec::new(),
            grounding: Some(Grounding::Enabled),
            response_prefix: None,
            json_schema: None,
            json_mode: false,
            additional_template_fields: BTreeMap::new(),
            escaped_special_tokens: BTreeMap::new(),
        }
    }
}

pub fn render_cmd3(opts: &RenderCmd3Options) -> Result<String, Box<dyn Error>> {
    let template_tools = tools_to_template(&opts.available_tools)?;
    let messages = messages_to_template(
        &opts.messages,
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
    substitutions.insert(
        "available_tools".to_string(),
        Value::Array(template_tools.into_iter().map(Value::Object).collect()),
    );
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
        .build()
        .unwrap()
        .parse(&opts.template)
        .unwrap();

    Ok(template.render(&liquid::object!(&substitutions)).unwrap())
}

pub fn render_cmd4(opts: &RenderCmd4Options) -> Result<String, Box<dyn Error>> {
    let template_tools = tools_to_template(&opts.available_tools)?;
    let messages = messages_to_template(
        &opts.messages,
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
    substitutions.insert(
        "available_tools".to_string(),
        Value::Array(template_tools.into_iter().map(Value::Object).collect()),
    );
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
        .build()
        .unwrap()
        .parse(&opts.template)
        .unwrap();

    Ok(template.render(&liquid::object!(&substitutions)).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fs;
    use std::path::Path;
    use pretty_assertions::assert_eq;

    fn read_test_cases(version: &str) -> Vec<(String, Value, String)> {
        let mut cases = vec![];
        let cur_file = file!();
        let cur_dir = Path::new(cur_file).parent().unwrap();
        let test_dir = cur_dir.join("tests").join(version);
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
        // for now always set the template to cmd3v1.
        let cmd3v1_template = include_str!("templates/cmd3-v1.tmpl").to_string();
        for (test_name, input_json, expected) in read_test_cases("cmd3") {
            let mut opts = deserialize::<_, RenderCmd3Options>(&input_json).unwrap();
            opts.template = cmd3v1_template.clone();
            let rendered = render_cmd3(&opts).unwrap();
            assert_eq!(
                rendered.trim(),
                expected.trim(),
                "Failed test: {}",
                test_name
            );
        }
    }

    #[test]
    fn test_render_cmd4_from_dir() {
        for (test_name, input_json, expected) in read_test_cases("cmd4") {
            let opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(
                rendered.trim(),
                expected.trim(),
                "Failed test: {}",
                test_name
            );
        }
    }
}
