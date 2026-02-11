use crate::errors::MelodyError;
use crate::templating::types::{
    CitationQuality, Document, Grounding, Message, ReasoningType, SafetyMode, Tool,
};
use crate::templating::util::{
    add_spaces_to_json_encoding, escape_special_tokens, messages_to_template, tools_to_template, tools_to_template_jinja, docs_to_template, docs_to_template_jinja
};
use minijinja::Environment;
use serde::Deserialize;
use serde_json::{Map, Value, json, to_string};
use std::collections::BTreeMap;

/// Options for cmd3 rendering.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RenderCmd3Options<'a> {
    /// Messages to include in the rendered prompt.
    pub messages: Vec<Message>,
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
static CMD3V1_TEMPLATE: &str = include_str!("templates/cmd3-v1.tmpl");
static CMD3_JINJA_TEMPLATE_BASE: &str =
    include_str!("templates/jinja/cmd3/chat_merged_template.jinja");
static CMD3V1_JINJA_TEMPLATE: &str =
    include_str!("templates/jinja/cmd3/chat_merged_template_v1.jinja");
static CMD3V3_JINJA_TEMPLATE: &str =
    include_str!("templates/jinja/cmd3/chat_merged_template_default_thinking.jinja");

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
    /// Messages to include in the rendered prompt.
    pub messages: Vec<Message>,
    /// Template string to use for rendering.
    pub template: &'a str,
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
        .map(|s| minijinja::Value::from_safe_string(add_spaces_to_json_encoding(&s)))
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

fn convert_messages_for_jinja(messages: &[Value]) -> Result<Vec<Value>, MelodyError> {
    fn get_vec<'a>(val_map: &'a mut Map<String, Value>, key: &str, def_val: &'a mut Value, def_vec: &'a mut Vec<Value>) -> &'a mut Vec<Value> {
        let val_vec = val_map
            .get_mut(key)
            .unwrap_or(def_val)
            .as_array_mut()
            .unwrap_or(def_vec);
        val_vec
    }

    let mut new_messages = vec![];
    let converted_messages = messages.iter().enumerate()
        .map(|(msg_idx, m)| -> Result<Value, MelodyError> {
            let mut new_m = m.clone();
            if let Some(mobj) = new_m.as_object_mut() {
                let def_str = json!("");
                let mut def_val = Value::Null;
                let mut def_vec = Vec::<Value>::new();
                let mobj_tmp = mobj.clone();
                let role = mobj_tmp.get("role").unwrap_or( &def_str).as_str().unwrap_or_default();

                let tool_calls = get_vec( mobj, "tool_calls", &mut def_val, &mut def_vec);
                let has_tool_calls = !tool_calls.is_empty();
                for t in tool_calls.iter_mut() {
                    let t_str = t.as_str().unwrap_or_default();
                    if t_str.is_empty() {
                        continue;
                    }
                    let tool_call: Map<String, Value> = serde_json::from_str(t_str)?;
                     *t = json!({
                        "id": tool_call.get("tool_call_id"),
                        "type": "function",
                        "function": {
                            "name": tool_call.get("tool_name"),
                            "arguments": tool_call.get("parameters")
                        }
                    })
                }

                let content = get_vec( mobj, "content", &mut def_val, &mut def_vec);
                for (content_idx, c) in content.iter_mut().enumerate() {
                    let mut def_map = Map::new();
                    let content_item = c.as_object_mut().unwrap_or(&mut def_map);
                    if role != "Tool" {
                        if let Some(content_type) = content_item.get("type") {
                            let mut type_str = content_type.as_str().unwrap_or_default().to_string();
                            if type_str == "text" && content_idx == 0 && has_tool_calls {
                                type_str = "thinking".to_string();
                                content_item.insert("type".to_string(), Value::String(type_str.clone()));
                            }
                            let data = content_item.get("data").unwrap_or_default();
                            content_item.insert(
                                type_str,
                                data.clone(),
                            );
                        }
                    }
                }

                let tool_results = get_vec( mobj, "tool_results", &mut def_val, &mut def_vec);
                let mut tool_call_to_new_msg: BTreeMap<i64, usize> = BTreeMap::new();
                for tres_val in tool_results.iter_mut() {
                    let def_map = Map::new();
                    let tres = tres_val.as_object().unwrap_or(&def_map);
                    let tool_call_id = tres.get("tool_call_id").unwrap_or_default().as_i64().ok_or(MelodyError::TemplateValidation("Invalid tool call id in results during jinja conversion".to_string()))?;
                    let documents = tres.get("documents").unwrap_or_default().as_array().ok_or(MelodyError::TemplateValidation("Invalid tool result documents during jinja conversion".to_string()))?;
                    if !tool_call_to_new_msg.contains_key(&tool_call_id) {
                        let new_msg = json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "content": Value::Array(Vec::new()),
                        });
                        new_messages.push((msg_idx, new_msg));
                        tool_call_to_new_msg.insert(tool_call_id, new_messages.len()-1);
                    }
                    let new_msg_idx = tool_call_to_new_msg[&tool_call_id];
                    let (_, msg_ref) = &mut new_messages[new_msg_idx];
                    for doc in documents {
                        let doc_str = doc.as_str().ok_or(MelodyError::TemplateValidation("Invalid tool document format during jinja conversion".to_string()))?;
                        msg_ref.get_mut("content").unwrap().as_array_mut().unwrap().push(serde_json::from_str(doc_str)?)
                    }
                }
            }
            Ok(new_m)
        })
        .collect::<Result<Vec<Value>, MelodyError>>()?;
        if new_messages.is_empty() {
            return Ok(converted_messages);
        }

        let msgs_len = messages.len();
        let mut new_msg_idx1 = new_messages.len();
        let mut all_msgs_rev = Vec::with_capacity(msgs_len + new_messages.len());
        for (msg_rev_idx, msg) in converted_messages.iter().rev().enumerate() {
            let msg_idx = msgs_len - msg_rev_idx - 1;
            let mut was_replaced = false;
            while new_msg_idx1 > 0 && let (insrt_idx, new_msg) = &new_messages[new_msg_idx1-1] && insrt_idx == &msg_idx {
                all_msgs_rev.push(new_msg.clone());
                was_replaced = true;
                new_msg_idx1 -= 1;
            }
            if !was_replaced {
                all_msgs_rev.push(msg.clone());
            }
        }
        all_msgs_rev.reverse();
        Ok(all_msgs_rev)
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
    let mut messages = messages_to_template(
        &opts.messages,
        !opts.documents.is_empty(),
        &opts.escaped_special_tokens,
    )?;
    let mut docs = docs_to_template(&opts.documents, &opts.escaped_special_tokens)?;

    if opts.use_jinja {
        docs = docs_to_template_jinja(&opts.documents, &opts.escaped_special_tokens)?;
        messages = convert_messages_for_jinja(&messages)?;
        template_tools = tools_to_template_jinja(&opts.available_tools)?;
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
        Value::Array(docs),
    );
    substitutions.insert(
        "available_tools".to_string(),
        Value::Array(template_tools.clone().into_iter().map(Value::Object).collect()),
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
        // TODO The next two substitutions should be configurable if used with vllm
        substitutions.insert("add_generation_prompt".to_string(), Value::Bool(true));
        substitutions.insert("bos_token".to_string(), json!("<BOS_TOKEN>"));
        substitutions.insert("regen_tool_call_ids".to_string(), json!(false));

        substitutions.insert("tools".to_string(), substitutions.get("available_tools").unwrap_or_default().clone());
        substitutions.insert("developer_preamble".to_string(), substitutions.get("preamble").unwrap_or_default().clone());
        if opts.citation_quality.as_ref().is_none_or(|v| *v != CitationQuality::Off) {
            substitutions.insert("enable_citations".to_string(), json!(true));
        }
        if opts.reasoning_type.is_some() {
            let reasoning_enabled = matches!(opts.reasoning_type, Some(ReasoningType::Enabled));
            substitutions.insert("reasoning".to_string(), Value::Bool(reasoning_enabled));
        }
        if opts.json_mode || opts.json_schema.is_some() {
            let mut json_val = json!({"type": "json_object"});
            if let Some(json_schema) = &opts.json_schema {
                json_val = json!({
                    "type": "json_object",
                    "schema": json_schema
                });
            }
            substitutions.insert("response_format".to_string(), json_val);
        }

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

/// Renders a CMD4 format prompt from the given options.
///
/// # Errors
///
/// Returns a `MelodyError` if:
/// - JSON serialization of documents fails
/// - Template parsing fails
/// - Template rendering fails
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
    fn test_render_cmd4_from_dir() {
        for (test_name, input_json, expected) in read_test_cases("cmd4") {
            println!("Running cmd4 test case: {}", test_name);
            let opts = deserialize::<_, RenderCmd4Options>(&input_json).unwrap();
            let rendered = render_cmd4(&opts).unwrap();
            assert_eq!(expected, rendered, "Failed test: {}", test_name);
        }
    }
}
