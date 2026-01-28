use crate::errors::MelodyError;
use crate::templating::types::{
    CitationQuality, Document, Grounding, Message, ReasoningType, SafetyMode, Tool,
};
use crate::templating::util::{
    add_spaces_to_json_encoding, escape_special_tokens, messages_to_template, tools_to_template,
};
use minijinja::Environment;
use serde::Deserialize;
use serde_json::{Map, Value, to_string};
use std::collections::BTreeMap;

/// Options for cmd3 rendering.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RenderCmd3Options<'a> {
    pub messages: Vec<Message>,
    pub template: &'a str,
    pub template_jinja: &'a str,
    pub use_jinja: bool,
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
    pub additional_template_fields: Map<String, Value>,
    pub escaped_special_tokens: BTreeMap<String, String>,
}
// for now always set the template to cmd3v1.
static CMD3V1_TEMPLATE: &str = include_str!("templates/cmd3-v1.tmpl");
static CMD3_JINJA_TEMPLATE_BASE: &str =
    include_str!("templates/jinja/cmd3/chat_merged_template.jinja");
static CMD3V1_JINJA_TEMPLATE: &str =
    include_str!("templates/jinja/cmd3/chat_merged_template_v1.jinja");

impl Default for RenderCmd3Options<'_> {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
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
    pub messages: Vec<Message>,
    pub template: &'a str,
    pub use_jinja: bool,
    pub dev_instruction: Option<String>,
    pub platform_instruction: Option<String>,
    pub documents: Vec<Document>,
    pub available_tools: Vec<Tool>,
    pub grounding: Option<Grounding>,
    pub response_prefix: Option<String>,
    pub json_schema: Option<String>,
    pub json_mode: bool,
    pub additional_template_fields: Map<String, Value>,
    pub escaped_special_tokens: BTreeMap<String, String>,
}

static CMD4V1_TEMPLATE: &str = include_str!("templates/cmd4-v1.tmpl");
impl Default for RenderCmd4Options<'_> {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            template: CMD4V1_TEMPLATE,
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

fn tojson(value: &minijinja::Value) -> Result<minijinja::Value, minijinja::Error> {
    // Based off of the minijinja version: https://github.com/mitsuhiko/minijinja/blob/64d933eaf325ba20e7af0012505571d7ae32364a/minijinja/src/filters.rs#L991
    // but we don't need indenting and we don't want the html char conversion, so using this
    serde_json::to_string(&value)
        .map_err(|err| {
            minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                "cannot serialize to JSON",
            )
            .with_source(err)
        })
        .map(minijinja::Value::from_safe_string)
}

fn get_minijinja_env<'a>(
    template_name: &'a str,
    template: &'a str,
) -> Result<minijinja::Environment<'a>, minijinja::Error> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    env.add_filter("tojson", tojson);
    env.add_template(template_name, template)?;
    Ok(env)
}

pub fn render_cmd3(opts: &RenderCmd3Options) -> Result<String, MelodyError> {
    let template_tools = tools_to_template(&opts.available_tools)?;
    let mut messages = messages_to_template(
        &opts.messages,
        !opts.documents.is_empty(),
        &opts.escaped_special_tokens,
    )?;
    let docs: Vec<String> = opts
        .documents
        .iter()
        .map(|d| -> Result<String, MelodyError> {
            Ok(add_spaces_to_json_encoding(&escape_special_tokens(
                &to_string(d)?,
                &opts.escaped_special_tokens,
            )))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if opts.use_jinja {
        messages = messages
            .iter()
            .map(|m| -> Value {
                let mut new_m = m.clone();
                if let Some(mobj) = new_m.as_object_mut() {
                    let mut def_val = Value::Null;
                    let mut def_vec = Vec::<Value>::new();
                    let content = mobj
                        .get_mut("content")
                        .unwrap_or(&mut def_val)
                        .as_array_mut()
                        .unwrap_or(&mut def_vec);
                    for c in content.iter_mut() {
                        let mut def_map = Map::new();
                        let content_item = c.as_object_mut().unwrap_or(&mut def_map);
                        if let Some(content_type) = content_item.get("type") {
                            let data = content_item.get("data").unwrap_or_default();
                            content_item.insert(
                                content_type.as_str().unwrap_or_default().to_string(),
                                data.clone(),
                            );
                        }
                    }
                }
                new_m
            })
            .collect();
    }

    let mut substitutions = opts.additional_template_fields.clone();
    substitutions.insert(
        "preamble".to_string(),
        opts.dev_instruction
            .clone()
            .map_or(Value::Null, Value::String),
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
    if opts.response_prefix.is_some() || !opts.use_jinja {
        substitutions.insert(
            "response_prefix".to_string(),
            opts.response_prefix
                .clone()
                .map_or(Value::Null, Value::String),
        );
    }
    substitutions.insert(
        "json_schema".to_string(),
        opts.json_schema.clone().map_or(Value::Null, Value::String),
    );
    substitutions.insert("json_mode".to_string(), Value::Bool(opts.json_mode));

    if opts.use_jinja {
        substitutions.insert("add_generation_prompt".to_string(), Value::Bool(true));
        let template_name = "chat_template.jinja";
        let mut env = get_minijinja_env(template_name, opts.template_jinja)?;
        env.add_template("chat_merged_template.jinja", CMD3_JINJA_TEMPLATE_BASE)?;
        let template = env.get_template(template_name)?;
        let template_str = template.render(&substitutions)?;

        Ok(template_str)
    } else {
        let parser = liquid::ParserBuilder::with_stdlib().build()?;
        let template = parser.parse(opts.template)?;

        Ok(template.render(&liquid::object!(&substitutions))?)
    }
}

pub fn render_cmd4(opts: &RenderCmd4Options) -> Result<String, MelodyError> {
    let template_tools = tools_to_template(&opts.available_tools)?;
    let messages = messages_to_template(
        &opts.messages,
        !opts.documents.is_empty(),
        &opts.escaped_special_tokens,
    )?;
    let docs: Vec<String> = opts
        .documents
        .iter()
        .map(|d| -> Result<String, MelodyError> {
            Ok(add_spaces_to_json_encoding(&escape_special_tokens(
                &to_string(d)?,
                &opts.escaped_special_tokens,
            )))
        })
        .collect::<Result<Vec<_>, _>>()?;

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
            .map_or(Value::Null, |g| Value::String(g.as_str().to_string())),
    );
    if opts.response_prefix.is_some() || !opts.use_jinja {
        substitutions.insert(
            "response_prefix".to_string(),
            opts.response_prefix
                .clone()
                .map_or(Value::Null, Value::String),
        );
    }
    substitutions.insert(
        "json_schema".to_string(),
        opts.json_schema.clone().map_or(Value::Null, Value::String),
    );
    substitutions.insert("json_mode".to_string(), Value::Bool(opts.json_mode));

    let parser = liquid::ParserBuilder::with_stdlib().build()?;
    let template = parser.parse(opts.template)?;

    Ok(template.render(&liquid::object!(&substitutions))?)
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
    fn test_render_cmd4_from_dir() {
        for (test_name, input_json, expected) in read_test_cases("cmd4") {
            println!("Running cmd4 test case: {}", test_name);
            let opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }
}
