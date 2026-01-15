use crate::templating::types::{ContentType, Message, Role, Tool, ToolCall};
use serde_json::{Map, Value, to_string};
use std::collections::BTreeMap;

pub fn add_spaces_to_json_encoding(input: &str) -> String {
    let mut b = String::with_capacity(input.len());
    let mut in_string_literal = false;
    let mut last_char_is_backslash = false;
    for c in input.chars() {
        b.push(c);
        if !in_string_literal && (c == ',' || c == ':') {
            b.push(' ');
        }
        if c == '"' && !last_char_is_backslash {
            in_string_literal = !in_string_literal;
        }
        last_char_is_backslash = c == '\\' && !last_char_is_backslash;
    }
    b
}
pub fn json_escape_string(s: &str) -> String {
    let b = serde_json::to_string(s).unwrap_or_default();
    if b.len() < 2 {
        return String::new();
    }
    b[1..b.len() - 1].to_string()
}

pub fn escape_special_tokens(text: &str, special_token_map: &BTreeMap<String, String>) -> String {
    let mut result = text.to_string();
    for (special_token, replacement) in special_token_map {
        result = result.replace(special_token, replacement);
    }
    result
}

#[derive(Debug, Clone)]
pub struct TemplateContent {
    pub content_type: String,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct TemplateToolResult {
    pub tool_call_id: usize,
    pub documents: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TemplateMessage {
    pub role: String,
    pub tool_calls: Vec<String>,
    pub content: Vec<TemplateContent>,
    pub tool_results: Vec<TemplateToolResult>,
}

// Convert TemplateContent to map
fn content_to_map(cs: &[TemplateContent]) -> Vec<Value> {
    cs.iter()
        .map(|c| {
            let mut m = Map::new();
            m.insert("type".to_string(), Value::String(c.content_type.clone()));
            m.insert("data".to_string(), Value::String(c.data.clone()));
            Value::Object(m)
        })
        .collect()
}

// Convert TemplateToolResult to map
fn tool_result_to_map(trs: &[TemplateToolResult]) -> Vec<Value> {
    trs.iter()
        .map(|tr| {
            let mut m = Map::new();
            m.insert(
                "tool_call_id".to_string(),
                Value::Number(tr.tool_call_id.into()),
            );
            m.insert(
                "documents".to_string(),
                Value::Array(
                    tr.documents
                        .iter()
                        .map(|d| Value::String(d.clone()))
                        .collect(),
                ),
            );
            Value::Object(m)
        })
        .collect()
}

// Convert TemplateMessage to map
fn message_to_map(ms: &[TemplateMessage]) -> Vec<Value> {
    ms.iter()
        .map(|m| {
            let mut map: Map<String, Value> = Map::new();
            map.insert("role".to_string(), Value::String(m.role.clone()));
            map.insert(
                "tool_calls".to_string(),
                Value::Array(
                    m.tool_calls
                        .iter()
                        .map(|tc| Value::String(tc.clone()))
                        .collect(),
                ),
            );
            map.insert(
                "content".to_string(),
                Value::Array(content_to_map(&m.content)),
            );
            map.insert(
                "tool_results".to_string(),
                Value::Array(tool_result_to_map(&m.tool_results)),
            );
            Value::Object(map)
        })
        .collect()
}

// Custom type for raw JSON parameters which omits quotes when serialized
struct RawJsonString(String);

impl serde::Serialize for RawJsonString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Parse the underlying string into serde_json::Value and serialize that.
        // This makes the serializer emit proper JSON (no surrounding quotes).
        let val: serde_json::Value =
            serde_json::from_str(&self.0).map_err(serde::ser::Error::custom)?;
        val.serialize(serializer)
    }
}

// Helper struct for serializing the tool call with raw parameters
#[derive(serde::Serialize)]
struct ToolCallTemplate {
    tool_call_id: String,
    tool_name: String,
    parameters: RawJsonString,
}

// Convert ToolCall to template string
fn tool_call_to_template(tc: &ToolCall, tc_index: usize) -> Result<String, String> {
    let tpl = ToolCallTemplate {
        tool_call_id: tc_index.to_string(),
        tool_name: tc.name.clone(),
        parameters: RawJsonString(tc.parameters.clone()),
    };
    let rendered = serde_json::to_string(&tpl).map_err(|e| e.to_string())?;
    Ok(add_spaces_to_json_encoding(&rendered))
}

// Convert tools to template
pub fn tools_to_template(tools: &[Tool]) -> Result<Vec<Map<String, Value>>, String> {
    let mut template_tools: Vec<Map<String, Value>> = Vec::with_capacity(tools.len());
    for tool in tools {
        let schema = serde_json::to_string(&tool.parameters)
            .map(|s| add_spaces_to_json_encoding(&s))
            .map_err(|e| e.to_string())?;
        let mut def = Map::new();
        def.insert(
            "description".to_string(),
            Value::String(json_escape_string(&tool.description)),
        );
        def.insert("json_schema".to_string(), Value::String(schema));
        let mut tool_map = Map::new();
        tool_map.insert(
            "name".to_string(),
            Value::String(json_escape_string(&tool.name)),
        );
        tool_map.insert("definition".to_string(), Value::Object(def));
        template_tools.push(tool_map);
    }
    Ok(template_tools)
}

// Convert messages to template
#[allow(clippy::too_many_lines)] //TODO: Refactor this function to reduce its length.
pub fn messages_to_template(
    messages: &[Message],
    docs_present: bool,
    special_token_map: &BTreeMap<String, String>,
) -> Result<Vec<Value>, String> {
    let mut template_messages: Vec<TemplateMessage> = Vec::new();
    let mut running_tool_call_idx = usize::from(docs_present);
    let mut tool_call_id_to_tool_result_idx = BTreeMap::new();
    let mut tool_call_id_to_prompt_id = BTreeMap::new();

    for (i, msg) in messages.iter().enumerate() {
        if msg.role == Role::Tool {
            let tool_call_id = msg
                .tool_call_id
                .as_ref()
                .ok_or_else(|| format!("tool message[{i}] missing tool_call_id"))?;
            let tool_call_template_id = *tool_call_id_to_prompt_id
                .entry(tool_call_id.clone())
                .or_insert_with(|| {
                    let idx = running_tool_call_idx;
                    running_tool_call_idx += 1;
                    idx
                });

            if template_messages.is_empty()
                || template_messages.last().unwrap().role != Role::Tool.as_str()
            {
                template_messages.push(TemplateMessage {
                    role: Role::Tool.as_str().to_string(),
                    tool_calls: vec![],
                    content: vec![],
                    tool_results: vec![],
                });
            }
            let m = template_messages.last_mut().unwrap();
            let tool_result_idx = *tool_call_id_to_tool_result_idx
                .entry(tool_call_id.clone())
                .or_insert_with(|| {
                    m.tool_results.push(TemplateToolResult {
                        tool_call_id: tool_call_template_id,
                        documents: vec![],
                    });
                    m.tool_results.len() - 1
                });

            for (j, content_item) in msg.content.iter().enumerate() {
                if content_item.content_type == ContentType::Text {
                    if let Some(ref text) = content_item.text {
                        let mut obj: Map<String, Value> = Map::new();
                        obj.insert("content".to_string(), Value::String(text.clone()));
                        let rendered_obj = add_spaces_to_json_encoding(
                            &to_string(&obj).map_err(|e| e.to_string())?,
                        );
                        m.tool_results[tool_result_idx]
                            .documents
                            .push(escape_special_tokens(&rendered_obj, special_token_map));
                    }
                } else if content_item.content_type == ContentType::Document {
                    if let Some(ref obj) = content_item.document {
                        let rendered_obj = add_spaces_to_json_encoding(
                            &to_string(obj).map_err(|e| e.to_string())?,
                        );
                        m.tool_results[tool_result_idx]
                            .documents
                            .push(escape_special_tokens(&rendered_obj, special_token_map));
                    }
                } else {
                    return Err(format!(
                        "tool message[{i}].content[{j}] invalid content type"
                    ));
                }
            }

            continue;
        }

        let mut template_msg_content = Vec::new();
        for content_item in &msg.content {
            match content_item.content_type {
                ContentType::Document => {
                    if msg.role != Role::Tool {
                        return Err("content type object is not supported for non-tool messages"
                            .to_string());
                    }
                    let data = if let Some(ref obj) = content_item.document {
                        let serialized = serde_json::to_string(obj).map_err(|e| e.to_string())?;
                        add_spaces_to_json_encoding(&serialized)
                    } else {
                        "{}".to_string()
                    };
                    template_msg_content.push(TemplateContent {
                        content_type: "text".to_string(),
                        data: escape_special_tokens(&data, special_token_map),
                    });
                }
                ContentType::Text => {
                    let data = if msg.role == Role::System {
                        content_item.text.clone().unwrap_or_default()
                    } else {
                        escape_special_tokens(
                            content_item.text.as_deref().unwrap_or_default(),
                            special_token_map,
                        )
                    };
                    template_msg_content.push(TemplateContent {
                        content_type: "text".to_string(),
                        data,
                    });
                }
                ContentType::Thinking => {
                    if msg.role == Role::Tool {
                        return Err(
                            "content type thinking is not supported for tool messages".to_string()
                        );
                    }
                    template_msg_content.push(TemplateContent {
                        content_type: "thinking".to_string(),
                        data: escape_special_tokens(
                            content_item.thinking.as_deref().unwrap_or_default(),
                            special_token_map,
                        ),
                    });
                }
                ContentType::Image => {
                    if msg.role == Role::Tool {
                        return Err(
                            "content type image is not supported for tool messages".to_string()
                        );
                    }
                    template_msg_content.push(TemplateContent {
                        content_type: "image".to_string(),
                        data: content_item
                            .image
                            .as_ref()
                            .map(|img| img.template_placeholder.clone())
                            .unwrap_or_default(),
                    });
                }
                ContentType::Unknown => {}
            }
        }

        let mut rendered_tool_calls = Vec::new();
        for tc in &msg.tool_calls {
            if msg.role != Role::Chatbot {
                return Err(
                    "tool calls are only supported for chatbot/assistant messages".to_string(),
                );
            }
            if tc.id.is_empty() {
                return Err(format!("message[{i}] has tool call with empty id"));
            }
            if tool_call_id_to_prompt_id.contains_key(&tc.id) {
                return Err(format!(
                    "message[{i}] has duplicate tool call id: {}",
                    tc.id
                ));
            }
            tool_call_id_to_prompt_id.insert(tc.id.clone(), running_tool_call_idx);
            let rendered_tool_call = tool_call_to_template(tc, running_tool_call_idx)?;
            running_tool_call_idx += 1;
            rendered_tool_calls.push(rendered_tool_call);
        }

        template_messages.push(TemplateMessage {
            role: msg.role.as_str().to_string(),
            tool_calls: rendered_tool_calls,
            content: template_msg_content,
            tool_results: vec![],
        });
    }
    Ok(message_to_map(&template_messages))
}
