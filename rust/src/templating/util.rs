use super::melody_types::*;
use liquid::model::{Object as LiquidObject, Value as LiquidValue};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// SafeLiquidSubstitutions converts any null values appropriately.
/// In Rust, we handle Option<T> differently than Go handles pointers.
pub fn safe_liquid_substitutions(mut sub: Map<String, Value>) -> Map<String, Value> {
    for (_k, v) in sub.iter_mut() {
        // In Rust, serde_json already handles null values properly
        // so we don't need the same transformation as Go
        if v.is_null() {
            *v = Value::Null;
        }
    }
    sub
}

/// Convert a serde_json::Value to a liquid::model::Value
pub fn json_to_liquid(value: Value) -> LiquidValue {
    match value {
        Value::Null => LiquidValue::Nil,
        Value::Bool(b) => LiquidValue::scalar(b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                LiquidValue::scalar(i)
            } else if let Some(f) = n.as_f64() {
                LiquidValue::scalar(f)
            } else {
                LiquidValue::Nil
            }
        }
        Value::String(s) => LiquidValue::scalar(s),
        Value::Array(arr) => {
            let liquid_arr: Vec<LiquidValue> = arr.into_iter().map(json_to_liquid).collect();
            LiquidValue::Array(liquid_arr)
        }
        Value::Object(obj) => {
            let mut liquid_obj = LiquidObject::new();
            for (k, v) in obj {
                liquid_obj.insert(k.into(), json_to_liquid(v));
            }
            LiquidValue::Object(liquid_obj)
        }
    }
}

/// AddSpacesToJSONEncoding adds spacing to a json string how command 3 models expect:
/// Spaces after colons in maps, and after commas.
pub fn add_spaces_to_json_encoding(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 2);
    let mut in_string_literal = false;
    let mut last_rune_is_backslash = false;

    for ch in input.chars() {
        result.push(ch);
        if !in_string_literal && (ch == ',' || ch == ':') {
            result.push(' ');
        }
        if ch == '"' && !last_rune_is_backslash {
            in_string_literal = !in_string_literal;
        }
        last_rune_is_backslash = ch == '\\' && !last_rune_is_backslash;
    }
    result
}

/// MarshalJSONFormatted will marshal the value to json with specific spacing for the prompt template
pub fn marshal_json_formatted(v: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let schema_string = serde_json::to_string_pretty(v)?;

    // Remove newlines but keep spaces as expected by the prompt template
    let schema_string_spaced = schema_string
        .replace("{\n", "{")
        .replace("\n}", "}")
        .replace("[\n", "[")
        .replace("\n]", "]")
        .replace('\n', " ");

    Ok(schema_string_spaced)
}

/// JSONEscapeString escapes a string for JSON
pub fn json_escape_string(s: &str) -> String {
    match serde_json::to_string(s) {
        Ok(escaped) => {
            // Remove the surrounding quotes
            if escaped.len() >= 2 {
                escaped[1..escaped.len() - 1].to_string()
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}

#[derive(Debug, Clone)]
struct TemplateContent {
    r#type: String,
    data: String,
}

#[derive(Debug, Clone)]
struct TemplateToolResult {
    tool_call_id: i32,
    documents: Vec<String>,
}

#[derive(Debug, Clone)]
struct TemplateMessage {
    role: String,
    tool_calls: Vec<String>,
    content: Vec<TemplateContent>,
    tool_results: Vec<TemplateToolResult>,
    additional_fields: HashMap<String, Value>,
}

fn message_to_map(ms: Vec<TemplateMessage>) -> Vec<Map<String, Value>> {
    ms.into_iter()
        .map(|m| {
            let mut mapped = Map::new();

            // Copy additional fields first
            for (k, v) in m.additional_fields {
                mapped.insert(k, v);
            }

            // Add standard fields
            mapped.insert("role".to_string(), Value::String(m.role));
            mapped.insert(
                "tool_calls".to_string(),
                Value::Array(m.tool_calls.into_iter().map(Value::String).collect()),
            );
            mapped.insert("content".to_string(), content_to_value(m.content));
            mapped.insert(
                "tool_results".to_string(),
                tool_result_to_value(m.tool_results),
            );

            mapped
        })
        .collect()
}

fn content_to_value(cs: Vec<TemplateContent>) -> Value {
    Value::Array(
        cs.into_iter()
            .map(|c| {
                json!({
                    "type": c.r#type,
                    "data": c.data,
                })
            })
            .collect(),
    )
}

fn tool_result_to_value(trs: Vec<TemplateToolResult>) -> Value {
    Value::Array(
        trs.into_iter()
            .map(|tr| {
                json!({
                    "tool_call_id": tr.tool_call_id,
                    "documents": tr.documents,
                })
            })
            .collect(),
    )
}

/// EscapeSpecialTokens replaces special tokens with their escaped versions
pub fn escape_special_tokens(text: &str, special_token_map: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (special_token, replacement) in special_token_map {
        result = result.replace(special_token, replacement);
    }
    result
}

/// MessagesToTemplate turns messages into a map that can be used with Command templates
pub fn messages_to_template(
    messages: Vec<Message>,
    docs_present: bool,
    special_token_map: &HashMap<String, String>,
) -> Result<Vec<Map<String, Value>>, Box<dyn std::error::Error>> {
    let mut template_messages: Vec<TemplateMessage> = Vec::new();
    let mut running_tool_call_idx = if docs_present { 1 } else { 0 };
    let mut tool_call_id_to_tool_result_idx: HashMap<String, usize> = HashMap::new();
    let mut tool_call_id_to_prompt_id: HashMap<String, i32> = HashMap::new();

    for (i, msg) in messages.iter().enumerate() {
        if msg.role == Role::Tool {
            // Template expects all tool messages to be aggregated
            let tool_call_id = &msg.tool_call_id;
            if tool_call_id.is_empty() {
                return Err(format!("tool message[{}] missing tool_call_id", i).into());
            }

            let tool_call_template_id = *tool_call_id_to_prompt_id
                .entry(tool_call_id.clone())
                .or_insert_with(|| {
                    let id = running_tool_call_idx;
                    running_tool_call_idx += 1;
                    id
                });

            // Aggregate text content into single string
            let mut tool_document = String::new();
            for (j, content_item) in msg.content.iter().enumerate() {
                if content_item.content_type != ContentType::Text {
                    return Err(format!(
                        "tool message[{}].content[{}] invalid content type: {}",
                        i, j, content_item.content_type
                    )
                    .into());
                }
                tool_document.push_str(&content_item.text);
            }

            // Insert a tool message to aggregate into (if it's the first message, or we haven't inserted a tool message yet for this hop)
            if template_messages.is_empty()
                || template_messages.last().unwrap().role != Role::Tool.to_string()
            {
                template_messages.push(TemplateMessage {
                    role: msg.role.to_string(),
                    tool_calls: Vec::new(),
                    content: Vec::new(),
                    tool_results: Vec::new(),
                    additional_fields: HashMap::new(),
                });
            }

            let m = template_messages.last_mut().unwrap();

            // Insert a tool result if one doesn't exist
            let tool_result_idx = match tool_call_id_to_tool_result_idx.get(tool_call_id) {
                Some(&idx) => idx,
                None => {
                    m.tool_results.push(TemplateToolResult {
                        tool_call_id: tool_call_template_id,
                        documents: Vec::new(),
                    });
                    let idx = m.tool_results.len() - 1;
                    tool_call_id_to_tool_result_idx.insert(tool_call_id.clone(), idx);
                    idx
                }
            };

            // Append the document to the tool result
            m.tool_results[tool_result_idx]
                .documents
                .push(escape_special_tokens(&tool_document, special_token_map));

            continue; // non-tool messages are handled below
        }

        let mut template_msg_content = Vec::new();
        for content_item in &msg.content {
            match content_item.content_type {
                ContentType::Text => {
                    let data = if msg.role == Role::System {
                        // don't escape special tokens for system messages
                        content_item.text.clone()
                    } else {
                        escape_special_tokens(&content_item.text, special_token_map)
                    };
                    template_msg_content.push(TemplateContent {
                        r#type: "text".to_string(),
                        data,
                    });
                }
                ContentType::Thinking => {
                    if msg.role == Role::Tool {
                        return Err(
                            "content type thinking is not supported for tool messages".into()
                        );
                    }
                    template_msg_content.push(TemplateContent {
                        r#type: "thinking".to_string(),
                        data: escape_special_tokens(&content_item.thinking, special_token_map),
                    });
                }
                ContentType::ImageUrl => {
                    if msg.role == Role::Tool {
                        return Err("content type image is not supported for tool messages".into());
                    }
                    if let Some(image) = &content_item.image {
                        template_msg_content.push(TemplateContent {
                            r#type: "image".to_string(),
                            data: image.template_placeholder.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        let mut rendered_tool_calls = Vec::new();
        for tc in &msg.tool_calls {
            if msg.role != Role::Chatbot {
                return Err("tool calls are only supported for chatbot/assistant messages".into());
            }
            if tc.id.is_empty() {
                return Err(format!("message[{}] has tool call with empty id", i).into());
            }
            if tool_call_id_to_prompt_id.contains_key(&tc.id) {
                return Err(format!("message[{}] has duplicate tool call id: {}", i, tc.id).into());
            }
            tool_call_id_to_prompt_id.insert(tc.id.clone(), running_tool_call_idx);
            let rendered_tool_call = tool_call_to_template(tc, running_tool_call_idx)?;
            running_tool_call_idx += 1;
            rendered_tool_calls.push(rendered_tool_call);
        }

        template_messages.push(TemplateMessage {
            role: msg.role.to_string(),
            tool_calls: rendered_tool_calls,
            content: template_msg_content,
            tool_results: Vec::new(),
            additional_fields: msg.additional_fields.clone(),
        });
    }

    Ok(message_to_map(template_messages))
}

fn tool_call_to_template(
    tc: &ToolCall,
    tc_index: i32,
) -> Result<String, Box<dyn std::error::Error>> {
    let tool_call_json = json!({
        "tool_call_id": tc_index.to_string(),
        "tool_name": tc.name,
        "parameters": tc.parameters,
    });

    let rendered = serde_json::to_string(&tool_call_json)?;
    Ok(add_spaces_to_json_encoding(&rendered))
}

/// ToolsToTemplate converts tools to template format
pub fn tools_to_template(tools: &[Tool]) -> Result<Vec<Map<String, Value>>, Box<dyn std::error::Error>> {
    let mut template_tools = Vec::new();

    for tool in tools {
        let formatted_json_schema = marshal_json_formatted(&Value::Object(
            tool.parameters.map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        ))?;

        let mut tool_map = Map::new();
        tool_map.insert("name".to_string(), json!(json_escape_string(&tool.name)));

        let mut definition = Map::new();
        definition.insert("description".to_string(), json!(json_escape_string(&tool.description)));
        definition.insert("json_schema".to_string(), json!(formatted_json_schema));

        tool_map.insert("definition".to_string(), Value::Object(definition));
        template_tools.push(tool_map);
    }

    Ok(template_tools)
}
