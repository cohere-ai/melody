use crate::errors::MelodyError;
use crate::parsing::types::FilterCitation;
use crate::templating::types::{ContentType, Document, Message, Role, Tool, ToolCall};
use crate::templating::{
    CitationQuality, Content, Grounding, ReasoningType, RenderCmd3Options, RenderCmd4Options,
};
use minijinja::Environment;
use serde_json::{Map, Value, json, to_string};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) fn add_spaces_to_json_encoding(input: &str) -> String {
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

pub(crate) fn json_escape_string(s: &str) -> String {
    let b = serde_json::to_string(s).unwrap_or_default();
    if b.len() < 2 {
        return String::new();
    }
    // drop the surrounding quotes since serde_json::to_string will add them.
    b[1..b.len() - 1].to_string()
}

pub(crate) fn escape_special_tokens(
    text: &str,
    special_token_map: &BTreeMap<String, String>,
) -> String {
    let mut result = text.to_string();
    for (special_token, replacement) in special_token_map {
        result = result.replace(special_token, replacement);
    }
    result
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateContent {
    pub content_type: String,
    pub data: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateToolResult {
    pub tool_call_id: usize,
    pub documents: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TemplateMessage {
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
fn tool_call_to_template(tc: &ToolCall, tc_index: usize) -> Result<String, MelodyError> {
    let tpl = ToolCallTemplate {
        tool_call_id: tc_index.to_string(),
        tool_name: tc.name.clone(),
        parameters: RawJsonString(tc.parameters.clone()),
    };
    let rendered = serde_json::to_string(&tpl)?;
    Ok(add_spaces_to_json_encoding(&rendered))
}

// Convert tools to template
pub(crate) fn tools_to_template(tools: &[Tool]) -> Result<Vec<Map<String, Value>>, MelodyError> {
    let mut template_tools: Vec<Map<String, Value>> = Vec::with_capacity(tools.len());
    for tool in tools {
        let schema =
            serde_json::to_string(&tool.parameters).map(|s| add_spaces_to_json_encoding(&s))?;
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

// Convert tools to template for jinja. Takes the input format of 'available_tools' and converts it to
// chat completions tool format: https://developers.openai.com/api/reference/resources/chat#(resource)%20chat.completions%20%3E%20(model)%20chat_completion_tool%20%3E%20(schema)
fn tools_to_template_jinja(tools: &[Tool]) -> Vec<Map<String, Value>> {
    let mut template_tools: Vec<Map<String, Value>> = Vec::with_capacity(tools.len());
    for tool in tools {
        let mut tool_map = Map::new();
        tool_map.insert("type".to_string(), Value::String("function".to_string()));
        let func = json!({
            "name": json_escape_string(&tool.name),
            "description": json_escape_string(&tool.description),
            "parameters": tool.parameters,
        });
        tool_map.insert("function".to_string(), func);
        template_tools.push(tool_map);
    }
    template_tools
}

fn escape_value_special_tokens(item: Value, special_token_map: &BTreeMap<String, String>) -> Value {
    match item {
        Value::String(s) => Value::String(escape_special_tokens(&s, special_token_map)),
        Value::Object(o) => Value::Object(escape_document_special_tokens(&o, special_token_map)),
        Value::Array(arr) => Value::Array(escape_array_special_tokens(&arr, special_token_map)),
        _ => item,
    }
}

fn escape_array_special_tokens(
    arr: &[Value],
    special_token_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    arr.iter()
        .map(|item| escape_value_special_tokens(item.clone(), special_token_map))
        .collect()
}

// Iterates recursively over the object and escapes any string values containing the special tokens.
fn escape_document_special_tokens(
    document: &Map<String, Value>,
    special_token_map: &BTreeMap<String, String>,
) -> Map<String, Value> {
    document
        .iter()
        .map(|(k, v)| {
            (
                escape_special_tokens(k, special_token_map),
                escape_value_special_tokens(v.clone(), special_token_map),
            )
        })
        .collect()
}

pub(crate) fn docs_to_template(
    documents: &[Map<String, Value>],
    special_token_map: &BTreeMap<String, String>,
) -> Result<Vec<Value>, MelodyError> {
    documents
        .iter()
        .map(|d| -> Result<_, MelodyError> {
            let escaped = &escape_document_special_tokens(d, special_token_map);
            Ok(Value::String(add_spaces_to_json_encoding(
                to_string(&escaped)?.as_str(),
            )))
        })
        .collect::<Result<Vec<_>, _>>()
}

fn docs_to_template_jinja(
    documents: &[Map<String, Value>],
    special_token_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    documents
        .iter()
        .map(|d| -> Value { Value::Object(escape_document_special_tokens(d, special_token_map)) })
        .collect()
}

fn build_text_with_citation(text: &String, citation_inserts: &mut [CitationInsertInfo]) -> String {
    fn get_cit_text(citation_insert: &CitationInsertInfo) -> String {
        if citation_insert.end {
            return format!("</co: {}>", citation_insert.id);
        }
        "<co>".to_string()
    }
    if citation_inserts.is_empty() {
        return text.clone();
    }
    // ascending sort
    citation_inserts.sort_by_key(|x| x.idx);
    let mut insert_cur_idx = 0;
    let mut new_text_builder = String::with_capacity(text.capacity());
    for (idx, char) in text.chars().enumerate() {
        let citation_insert = &citation_inserts[insert_cur_idx];
        if idx == citation_insert.idx {
            new_text_builder.push_str(&get_cit_text(citation_insert));
            while insert_cur_idx + 1 < citation_inserts.len()
                && citation_inserts[insert_cur_idx].idx == idx
            {
                insert_cur_idx += 1;
            }
        }
        new_text_builder.push(char);
    }
    let citation_insert = &citation_inserts[insert_cur_idx];
    if citation_insert.idx == text.len() {
        new_text_builder.push_str(&get_cit_text(citation_insert));
    }
    new_text_builder
}

struct CitationInsertInfo {
    idx: usize,
    end: bool,
    id: String,
}

fn add_citation_insert_pair(
    citation: &FilterCitation,
    citation_inserts: &mut Vec<CitationInsertInfo>,
) {
    let insrt_start = CitationInsertInfo {
        idx: citation.start_index,
        end: false,
        id: String::new(),
    };
    let mut citation_id_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for source in &citation.sources {
        citation_id_map
            .entry(source.tool_call_index)
            .or_default()
            .extend_from_slice(&source.tool_result_indices);
    }
    let mut citation_ids = Vec::new();
    for (tool_call_idx, result_ids) in citation_id_map {
        let citation_id = format!(
            "{tool_call_idx}:[{}]",
            result_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join(",")
        );
        citation_ids.push(citation_id);
    }
    let insrt_end = CitationInsertInfo {
        idx: citation.end_index,
        end: true,
        id: citation_ids.join(","),
    };

    citation_inserts.extend([insrt_start, insrt_end]);
}

fn tool_content_item_to_template(
    content_item: &Content,
    special_token_map: &BTreeMap<String, String>,
) -> Result<String, MelodyError> {
    match content_item.content_type {
        ContentType::Text => {
            if let Some(ref text) = content_item.text {
                let mut obj: Map<String, Value> = Map::new();
                let escaped_text = escape_special_tokens(text, special_token_map);
                obj.insert("content".to_string(), Value::String(escaped_text));
                return Ok(add_spaces_to_json_encoding(&to_string(&obj)?));
            }
            Err(MelodyError::TemplateValidation(
                "text content type must have text field".parse().unwrap(),
            ))
        }
        ContentType::Document => {
            if let Some(ref obj) = content_item.document {
                let escaped_object = escape_document_special_tokens(obj, special_token_map);
                return Ok(add_spaces_to_json_encoding(&to_string(&escaped_object)?));
            }
            Err(MelodyError::TemplateValidation(
                "document content type must have document field"
                    .parse()
                    .unwrap(),
            ))
        }
        ContentType::Image => {
            if let Some(ref obj) = content_item.image {
                let img_obj = json!({"image_content": obj.template_placeholder.clone()});
                return Ok(add_spaces_to_json_encoding(&to_string(&img_obj)?));
            }
            Err(MelodyError::TemplateValidation(
                "image content type must have image field".parse().unwrap(),
            ))
        }
        ContentType::Multipart => {
            if let Some(ref parts) = content_item.multipart {
                let mut part_strings: Vec<String> = Vec::with_capacity(parts.len());
                for part in parts {
                    if part.content_type == ContentType::Multipart {
                        return Err(MelodyError::TemplateValidation(
                            "multipart content cannot be nested in other multipart content"
                                .parse()
                                .unwrap(),
                        ));
                    }
                    if part.content_type == ContentType::Document {
                        return Err(MelodyError::TemplateValidation(
                            "document content cannot be nested in multipart content"
                                .parse()
                                .unwrap(),
                        ));
                    }
                    part_strings.push(tool_content_item_to_template(part, special_token_map)?);
                }
                return Ok(format!("[{}]", part_strings.join(", ")));
            }
            Err(MelodyError::TemplateValidation(
                "multipart content type must have multipart field"
                    .parse()
                    .unwrap(),
            ))
        }
        ContentType::Thinking => Err(MelodyError::TemplateValidation(
            "thinking content type cannot be used in tool messages"
                .parse()
                .unwrap(),
        )),
        ContentType::Unknown => Err(MelodyError::TemplateValidation(
            "invalid content type".parse().unwrap(),
        )),
    }
}

fn extract_doc_id(doc: &Document) -> String {
    doc.get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn extract_content_doc_id(content: &Content) -> String {
    match content.content_type {
        ContentType::Document => content
            .document
            .as_ref()
            .map(extract_doc_id)
            .unwrap_or_default(),
        ContentType::Multipart => content
            .multipart
            .as_ref()
            .and_then(|parts| parts.first())
            .and_then(|part| part.document.as_ref())
            .map(extract_doc_id)
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Identifier lookup tables produced when messages/documents are numbered
/// for a prompt.
///
/// The tables mirror the numeric axes emitted by the parser inside citations:
///
/// - `document_ids[tool_call_index][tool_result_index]` yields the original
///   `id` field of the document placed at that position in the prompt
///   (empty string when the source document had no `id`).
/// - `tool_call_ids[tool_call_index]` yields the original `tool_call_id`
///   string, or an empty string for the reserved bucket that holds the
///   top-level `documents` array (index `0` when that array is non-empty).
///
/// Both vectors always have the same length.
///
/// Produced by [`PromptRenderIds::from_messages`], and used by
/// [`crate::parsing::FilterOptions::with_message_history`] to resolve
/// citation indices back to their original document identifiers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PromptRenderIds {
    /// Original `tool_call_id` strings, indexed by `tool_call_index`.
    pub tool_call_ids: Vec<String>,
    /// Original document identifiers, indexed as
    /// `document_ids[tool_call_index][tool_result_index]`.
    pub document_ids: Vec<Vec<String>>,
}

impl PromptRenderIds {
    fn new(top_level_documents: &[Document]) -> Self {
        let mut ids = Self::default();
        if !top_level_documents.is_empty() {
            ids.tool_call_ids.push(String::new());
            ids.document_ids
                .push(top_level_documents.iter().map(extract_doc_id).collect());
        }
        ids
    }

    fn push_tool_call(&mut self, tool_call_id: &str) -> usize {
        let idx = self.tool_call_ids.len();
        self.tool_call_ids.push(tool_call_id.to_string());
        self.document_ids.push(Vec::new());
        idx
    }

    fn push_tool_message_contents(&mut self, tool_call_idx: usize, content: &[Content]) {
        if let Some(row) = self.document_ids.get_mut(tool_call_idx) {
            for c in content {
                row.push(extract_content_doc_id(c));
            }
        }
    }

    /// Walk `messages` and `documents` the same way the renderer would,
    /// producing the identifier lookup tables that describe how the
    /// templating engine will assign `tool_call_index` /
    /// `tool_result_index` axes.
    ///
    /// This is the single source of truth for prompt-side ID numbering.
    /// The renderer ([`messages_to_template`]) and the parser
    /// ([`crate::parsing::FilterOptions::with_message_history`]) both call
    /// through here so the two paths stay consistent.
    ///
    /// The function is best-effort: malformed inputs (a
    /// tool message with no `tool_call_id`, an empty or duplicate
    /// `ToolCall::id`, `tool_calls` on a non-`Chatbot` message, etc.) do
    /// not error here. They are added into the lookup
    /// — missing IDs become the empty string, duplicates reuse the
    /// existing bucket. Callers that care about template shape (i.e. the
    /// renderer) must validate separately; the parser deliberately does
    /// not.
    #[must_use]
    pub fn from_messages(messages: &[Message], documents: &[Document]) -> Self {
        let mut prompt_ids = Self::new(documents);
        let mut tool_call_id_to_idx = HashMap::<String, usize>::new();

        for msg in messages {
            if msg.role == Role::Tool {
                let tool_call_id = msg.tool_call_id.as_deref().unwrap_or_default();
                let idx = *tool_call_id_to_idx
                    .entry(tool_call_id.to_string())
                    .or_insert_with(|| prompt_ids.push_tool_call(tool_call_id));
                prompt_ids.push_tool_message_contents(idx, &msg.content);
                continue;
            }

            for tc in &msg.tool_calls {
                tool_call_id_to_idx
                    .entry(tc.id.clone())
                    .or_insert_with(|| prompt_ids.push_tool_call(&tc.id));
            }
        }

        prompt_ids
    }
}

/// Validate the shape of a single message so the templating engine can
/// serialise it unambiguously. Designed to be called once per message
/// inside the renderer's existing message walk, so template building
/// and validation share a single pass.
///
/// `seen_tool_call_ids` threads duplicate-detection state across
/// successive calls; pass a fresh empty set before the first message.
///
/// The parser deliberately does not call this — malformed histories are
/// the renderer's concern. See [`PromptRenderIds::from_messages`] for
/// the parser's best-effort numbering.
fn validate_message_for_rendering<'a>(
    msg: &'a Message,
    i: usize,
    seen_tool_call_ids: &mut HashSet<&'a str>,
) -> Result<(), MelodyError> {
    if msg.role == Role::Tool {
        if msg.tool_call_id.is_none() {
            return Err(MelodyError::TemplateValidation(format!(
                "tool message[{i}] missing tool_call_id"
            )));
        }
        return Ok(());
    }

    for tc in &msg.tool_calls {
        if msg.role != Role::Chatbot {
            return Err(MelodyError::TemplateValidation(
                "tool calls are only supported for chatbot/assistant messages".to_string(),
            ));
        }
        if tc.id.is_empty() {
            return Err(MelodyError::TemplateValidation(format!(
                "message[{i}] has tool call with empty id"
            )));
        }
        if !seen_tool_call_ids.insert(tc.id.as_str()) {
            return Err(MelodyError::TemplateValidation(format!(
                "message[{i}] has duplicate tool call id: {}",
                tc.id
            )));
        }
    }
    Ok(())
}

// Convert messages to template.
//
// Numbering is delegated to `PromptRenderIds::from_messages` so it stays a
// single source of truth; this function performs a second walk purely for
// template construction, looking up already-assigned indices instead of
// re-numbering.
#[allow(clippy::too_many_lines)] //TODO: Refactor this function to reduce its length.
pub(crate) fn messages_to_template(
    messages: &[Message],
    documents: &[Document],
    special_token_map: &BTreeMap<String, String>,
) -> Result<Vec<Value>, MelodyError> {
    let prompt_ids = PromptRenderIds::from_messages(messages, documents);

    // Precompute tool_call_id → prompt index, skipping the reserved
    // top-level documents bucket at index 0 (when present).
    let starting_idx = usize::from(!documents.is_empty());
    let tool_call_id_to_prompt_id: BTreeMap<&str, usize> = prompt_ids
        .tool_call_ids
        .iter()
        .enumerate()
        .skip(starting_idx)
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    let mut template_messages: Vec<TemplateMessage> = Vec::new();
    let mut tool_call_id_to_tool_result_idx = BTreeMap::new();
    let mut seen_tool_call_ids: HashSet<&str> = HashSet::new();

    for (i, msg) in messages.iter().enumerate() {
        validate_message_for_rendering(msg, i, &mut seen_tool_call_ids)?;

        if msg.role == Role::Tool {
            // `validate_message_for_rendering` checked tool_call_id is
            // present; `from_messages` assigned a bucket for every id it
            // saw (including this one).
            let tool_call_id = msg.tool_call_id.as_deref().unwrap_or_default();
            let tool_call_template_id = tool_call_id_to_prompt_id
                .get(tool_call_id)
                .copied()
                .expect("from_messages assigned every tool_call_id");

            if template_messages.is_empty()
                || template_messages
                    .last()
                    .is_none_or(|msg| msg.role != Role::Tool.as_str())
            {
                template_messages.push(TemplateMessage {
                    role: Role::Tool.as_str().to_string(),
                    tool_calls: vec![],
                    content: vec![],
                    tool_results: vec![],
                });
            }
            let m = template_messages.last_mut().ok_or_else(|| {
                MelodyError::TemplateValidation(
                    "Internal error: template_messages should not be empty".to_string(),
                )
            })?;
            let tool_result_idx = *tool_call_id_to_tool_result_idx
                .entry(tool_call_id.to_string())
                .or_insert_with(|| {
                    m.tool_results.push(TemplateToolResult {
                        tool_call_id: tool_call_template_id,
                        documents: vec![],
                    });
                    m.tool_results.len() - 1
                });

            for content_item in &msg.content {
                m.tool_results[tool_result_idx]
                    .documents
                    .push(tool_content_item_to_template(
                        content_item,
                        special_token_map,
                    )?);
            }

            continue;
        }

        let mut template_msg_content = Vec::new();
        for (j, content_item) in msg.content.iter().enumerate() {
            let mut citation_inserts = Vec::<CitationInsertInfo>::new();
            for citation in &msg.citations {
                // TODO Fix citation to use content index instead of is_thinking then can simplify this
                if msg.content.len() == 1
                    || citation.is_thinking && j == 0
                    || !citation.is_thinking && j == 1
                {
                    add_citation_insert_pair(citation, &mut citation_inserts);
                }
            }
            match content_item.content_type {
                ContentType::Document => {
                    if msg.role != Role::Tool {
                        return Err(MelodyError::TemplateValidation(
                            "content type object is not supported for non-tool messages"
                                .to_string(),
                        ));
                    }
                }
                ContentType::Multipart => {
                    if msg.role != Role::Tool {
                        return Err(MelodyError::TemplateValidation(
                            "content type multipart is not supported for non-tool messages"
                                .to_string(),
                        ));
                    }
                }
                ContentType::Text => {
                    let data = if msg.role == Role::System {
                        content_item.text.clone().unwrap_or_default()
                    } else {
                        build_text_with_citation(
                            &escape_special_tokens(
                                content_item.text.as_deref().unwrap_or_default(),
                                special_token_map,
                            ),
                            &mut citation_inserts,
                        )
                    };
                    template_msg_content.push(TemplateContent {
                        content_type: "text".to_string(),
                        data,
                    });
                }
                ContentType::Thinking => {
                    if msg.role == Role::Tool {
                        return Err(MelodyError::TemplateValidation(
                            "content type thinking is not supported for tool messages".to_string(),
                        ));
                    }
                    template_msg_content.push(TemplateContent {
                        content_type: "thinking".to_string(),
                        data: build_text_with_citation(
                            &escape_special_tokens(
                                content_item.thinking.as_deref().unwrap_or_default(),
                                special_token_map,
                            ),
                            &mut citation_inserts,
                        ),
                    });
                }
                ContentType::Image => {
                    if msg.role == Role::Tool {
                        return Err(MelodyError::TemplateValidation(
                            "content type image is not supported for tool messages".to_string(),
                        ));
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

        // `validate_messages_for_rendering` already rejected empty ids,
        // duplicates, and tool_calls on non-Chatbot roles.
        // `from_messages` assigned every id we look up here.
        let mut rendered_tool_calls = Vec::new();
        for tc in &msg.tool_calls {
            let tool_call_template_id = tool_call_id_to_prompt_id
                .get(tc.id.as_str())
                .copied()
                .expect("from_messages assigned every tool_call id");
            let rendered_tool_call = tool_call_to_template(tc, tool_call_template_id)?;
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

// Based off of the minijinja version: https://github.com/mitsuhiko/minijinja/blob/64d933eaf325ba20e7af0012505571d7ae32364a/minijinja/src/filters.rs#L991
// but we don't need indenting and we don't want the html char conversion, so using this
fn tojson(value: &minijinja::Value) -> Result<minijinja::Value, minijinja::Error> {
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

// Set the minijinja env according to the huggingface settings:
// https://github.com/huggingface/transformers/blob/57278c904c5158999d31a0db8bfcd63360c37b48/src/transformers/utils/chat_template_utils.py#L455-L460
pub(crate) fn get_minijinja_env<'a>(
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

// This function does the majority of work needed to convert from our internal message format to
// the jinja supported chat completions format
#[allow(clippy::too_many_lines)]
fn convert_messages_for_jinja(
    messages: &[Value],
    raw_tool_call_names: bool,
) -> Result<Vec<Value>, MelodyError> {
    fn get_vec<'a>(
        val_map: &'a mut Map<String, Value>,
        key: &str,
        def_val: &'a mut Value,
        def_vec: &'a mut Vec<Value>,
    ) -> &'a mut Vec<Value> {
        (val_map
            .get_mut(key)
            .unwrap_or(def_val)
            .as_array_mut()
            .unwrap_or(def_vec)) as _
    }

    // These are new messages due to tool results that we will insert after other conversions
    let mut new_messages = vec![];
    // First bit of code is just to be able to loop through the messages mutably
    let converted_messages = messages
        .iter()
        .enumerate()
        .map(|(msg_idx, m)| -> Result<Value, MelodyError> {
            let mut new_m = m.clone();
            if let Some(mobj) = new_m.as_object_mut() {
                let def_str = json!("");
                let mut def_val = Value::Null;
                let mut def_vec = Vec::<Value>::new();
                let mobj_tmp = mobj.clone();
                let role = mobj_tmp
                    .get("role")
                    .unwrap_or(&def_str)
                    .as_str()
                    .unwrap_or_default();

                // If there are tool calls in the message, we want to change the format to match chat completions:
                // https://developers.openai.com/api/reference/resources/chat#(resource)%20chat.completions%20%3E%20(model)%20chat_completion_message_tool_call%20%3E%20(schema)
                let tool_calls = get_vec(mobj, "tool_calls", &mut def_val, &mut def_vec);
                let has_tool_calls = !tool_calls.is_empty();
                for t in tool_calls.iter_mut() {
                    let t_str = t.as_str().unwrap_or_default();
                    if t_str.is_empty() {
                        continue;
                    }
                    let tool_call: Map<String, Value> = serde_json::from_str(t_str)?;
                    let tool_name = tool_call
                        .get("tool_name")
                        .unwrap_or_default()
                        .as_str()
                        .unwrap_or_default();
                    let name = if raw_tool_call_names {
                        tool_name.to_string()
                    } else {
                        json_escape_string(tool_name)
                    };
                    *t = json!({
                        "id": tool_call.get("tool_call_id"),
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": tool_call.get("parameters")
                        }
                    });
                }

                // This next section modifies the content array to use the appropriate chat completions field name
                // instead of "data" to match our v2 api: https://docs.cohere.com/reference/chat#request.body.messages
                // This mostly aligns with the chat completions format but we have our own thinking type
                let content = get_vec(mobj, "content", &mut def_val, &mut def_vec);
                for (content_idx, c) in content.iter_mut().enumerate() {
                    let mut def_map = Map::new();
                    let content_item = c.as_object_mut().unwrap_or(&mut def_map);
                    if role.to_lowercase() != "tool"
                        && let Some(content_type) = content_item.get("type")
                    {
                        let mut type_str = content_type.as_str().unwrap_or_default().to_string();
                        if type_str == "text" && content_idx == 0 && has_tool_calls {
                            type_str = "thinking".to_string();
                            content_item
                                .insert("type".to_string(), Value::String(type_str.clone()));
                        }
                        let data = content_item.get("data").unwrap_or_default();
                        content_item.insert(type_str, data.clone());
                    }
                }

                // Here we deal with tool_results which is the most complicated because while our liquid template has the
                // tool results of multiple tool calls in one array we have to split it out to a message per tool call id
                // to match the chat completions / our v2 format: https://docs.cohere.com/reference/chat#request.body.messages.Tool-Message
                // As a result this code creates a vector of new messages to insert and stores at which index to insert them
                let tool_results = get_vec(mobj, "tool_results", &mut def_val, &mut def_vec);
                // We build a map of tool call to new message index so that we can grab the correct existing 'new' message if the tool results
                // for some reason has the same tool call id in multiple array items or create a new one
                let mut tool_call_to_new_msg: BTreeMap<i64, usize> = BTreeMap::new();
                for tres_val in tool_results.iter_mut() {
                    let def_map = Map::new();
                    let tres = tres_val.as_object().unwrap_or(&def_map);
                    let tool_call_id = tres
                        .get("tool_call_id")
                        .unwrap_or_default()
                        .as_i64()
                        .ok_or(MelodyError::TemplateValidation(
                            "Invalid tool call id in results during jinja conversion".to_string(),
                        ))?;
                    // Get the documents to append to the new tool message content
                    let documents = tres.get("documents").unwrap_or_default().as_array().ok_or(
                        MelodyError::TemplateValidation(
                            "Invalid tool result documents during jinja conversion".to_string(),
                        ),
                    )?;
                    // Get the new tool message idx from the map for this tool call id or insert it if not present
                    let new_msg_idx =
                        tool_call_to_new_msg.entry(tool_call_id).or_insert_with(|| {
                            let new_msg = json!({
                                "role": "tool",
                                "tool_call_id": tool_call_id,
                                "content": Value::Array(Vec::new()),
                            });
                            new_messages.push((msg_idx, new_msg));
                            new_messages.len() - 1
                        });
                    // Get the new message itself
                    let (_, msg_ref) = &mut new_messages[*new_msg_idx];
                    // Append the documents to the message content
                    for doc in documents {
                        let doc_str = doc.as_str().ok_or(MelodyError::TemplateValidation(
                            "Invalid tool document format during jinja conversion".to_string(),
                        ))?;
                        let doc_obj: Value = serde_json::from_str(doc_str)?;
                        let doc_wrapper =
                            json!({"type": "document", "document": {"data": doc_obj}});
                        msg_ref
                            .get_mut("content")
                            .unwrap()
                            .as_array_mut()
                            .unwrap()
                            .push(doc_wrapper);
                    }
                }
            }
            Ok(new_m)
        })
        .collect::<Result<Vec<Value>, MelodyError>>()?;
    if new_messages.is_empty() {
        // There are no new messages to insert due to tool results, so just return the
        // messages converted for jinja
        return Ok(converted_messages);
    }

    // There are new tool messages to insert due to tool results
    let new_msgs_len = new_messages.len();
    let msgs_len = messages.len();
    // The index of the new message to insert
    let mut new_msg_idx = 0;
    // Allocate a new 'all_msgs' vector with the size of the existing messages plus new messages.
    // In reality we will skip some tool_results messages so this will be slightly oversized
    let mut all_msgs = Vec::with_capacity(msgs_len + new_msgs_len);
    // Iterate over the messages and insert new messages at the appropriate indexes
    for (msg_idx, msg) in converted_messages.iter().enumerate() {
        let mut was_replaced = false;
        // While there is a tool message to insert at this message index, loop and insert it
        while new_msg_idx < new_msgs_len
            && let (insrt_idx, new_msg) = &new_messages[new_msg_idx]
            && insrt_idx == &msg_idx
        {
            all_msgs.push(new_msg.clone());
            was_replaced = true;
            new_msg_idx += 1;
        }
        // If this was a tool results message that was replaced by individual tool role messages
        // then skip it, otherwise add it to all the messages
        if !was_replaced {
            all_msgs.push(msg.clone());
        }
    }
    Ok(all_msgs)
}

// Helper function to reduce duplication
#[allow(clippy::type_complexity)]
pub(crate) fn get_jinja_vars(
    messages: &[Value],
    tools: &[Tool],
    documents: &[Map<String, Value>],
    special_token_map: &BTreeMap<String, String>,
    raw_tool_call_names: bool,
) -> Result<(Vec<Value>, Vec<Map<String, Value>>, Vec<Value>), MelodyError> {
    let messages = convert_messages_for_jinja(messages, raw_tool_call_names)?;
    let template_tools = tools_to_template_jinja(tools);
    let docs = docs_to_template_jinja(documents, special_token_map);
    Ok((messages, template_tools, docs))
}

// Common jinja subsitutions for cmd3 and cmd4
#[allow(clippy::ref_option)]
pub(crate) fn add_jinja_substitutions_common(
    substitutions: &mut Map<String, Value>,
    json_mode: bool,
    json_schema: &Option<String>,
    reasoning_type: &Option<ReasoningType>,
) {
    // TODO The next two substitutions should be configurable if used with vllm
    substitutions.insert("add_generation_prompt".to_string(), Value::Bool(true));
    substitutions.insert("bos_token".to_string(), json!("<BOS_TOKEN>"));
    substitutions.insert("regen_tool_call_ids".to_string(), json!(false));
    substitutions.insert("convert_first_system_msg".to_string(), json!(false));

    substitutions.insert(
        "tools".to_string(),
        substitutions
            .get("available_tools")
            .unwrap_or_default()
            .clone(),
    );

    if reasoning_type.is_some() {
        let reasoning_enabled = matches!(reasoning_type, Some(ReasoningType::Enabled));
        substitutions.insert("reasoning".to_string(), Value::Bool(reasoning_enabled));
    }

    if json_mode || json_schema.is_some() {
        let mut json_val = json!({"type": "json_object"});
        if let Some(json_schema) = &json_schema {
            json_val = json!({
                "type": "json_object",
                "schema": json_schema
            });
        }
        substitutions.insert("response_format".to_string(), json_val);
    }
}

pub(crate) fn add_jinja_substitutions_cmd3(
    substitutions: &mut Map<String, Value>,
    opts: &RenderCmd3Options,
) {
    substitutions.insert(
        "developer_preamble".to_string(),
        substitutions.get("preamble").unwrap_or_default().clone(),
    );
    if opts
        .citation_quality
        .as_ref()
        .is_none_or(|v| *v != CitationQuality::Off)
    {
        substitutions.insert("enable_citations".to_string(), json!(true));
    }
}

pub(crate) fn add_jinja_substitutions_cmd4(
    substitutions: &mut Map<String, Value>,
    opts: &RenderCmd4Options,
) {
    substitutions.insert(
        "developer_preamble".to_string(),
        substitutions
            .get("developer_instruction")
            .unwrap_or_default()
            .clone(),
    );
    // TODO not currently used in cmd4 template but probably should be for backwards compatibility
    let grounding = opts
        .grounding
        .as_ref()
        .is_some_and(|x| *x == Grounding::Enabled);
    substitutions.insert("enable_citations".to_string(), Value::Bool(grounding));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_document_special_tokens_basic() {
        let mut special_token_map = BTreeMap::new();
        special_token_map.insert("a".to_string(), "o".to_string());

        let mut map = Map::new();
        map.insert("foofaa".to_string(), Value::String("borbar".to_string()));

        let escaped = escape_document_special_tokens(&map, &special_token_map);

        assert_eq!(escaped.get("foofoo").unwrap(), "borbor");
    }

    #[test]
    fn test_escape_document_special_tokens_nested() {
        let mut special_token_map = BTreeMap::new();
        special_token_map.insert("a".to_string(), "o".to_string());

        let inner_inner_arr = vec![
            Value::String("foofaa".to_string()),
            Value::Object({
                let mut m = Map::new();
                m.insert(
                    "inner_inner_key_aaa".to_string(),
                    Value::String("inner_inner_value_aaa".to_string()),
                );
                m
            }),
        ];

        let mut inner = Map::new();
        inner.insert("inner_key_aaa".to_string(), Value::Array(inner_inner_arr));

        let mut map = Map::new();
        map.insert(
            "outer_key".to_string(),
            Value::Array(vec![
                Value::String("zoozaa".to_string()),
                Value::Object(inner),
            ]),
        );

        let escaped = escape_document_special_tokens(&map, &special_token_map);
        let result = to_string(&escaped).unwrap();
        let expected = r#"{"outer_key":["zoozoo",{"inner_key_ooo":["foofoo",{"inner_inner_key_ooo":"inner_inner_volue_ooo"}]}]}"#;
        assert_eq!(result, expected);
    }

    mod test_prompt_render_ids_from_messages {
        use super::*;

        fn parse_messages(json: &str) -> Vec<Message> {
            let value: Value = serde_json::from_str(json).unwrap();
            serde_path_to_error::deserialize(&value).unwrap()
        }

        fn parse_documents(json: &str) -> Vec<Document> {
            let value: Value = serde_json::from_str(json).unwrap();
            serde_path_to_error::deserialize(&value).unwrap()
        }

        #[test]
        fn empty_inputs_produce_empty_tables() {
            let ids = PromptRenderIds::from_messages(&[], &[]);
            assert!(ids.tool_call_ids.is_empty());
            assert!(ids.document_ids.is_empty());
        }

        #[test]
        fn top_level_documents_only_reserve_index_zero() {
            let docs = parse_documents(r#"[{"id": "doc-a"}, {"id": "doc-b"}]"#);
            let ids = PromptRenderIds::from_messages(&[], &docs);
            assert_eq!(ids.tool_call_ids, vec![String::new()]);
            assert_eq!(
                ids.document_ids,
                vec![vec!["doc-a".to_string(), "doc-b".to_string()]]
            );
        }

        #[test]
        fn tool_calls_and_results() {
            let msgs = parse_messages(
                r#"[
                    {"role": "user", "content": [{"type": "text", "text": "hi"}]},
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"},
                        {"id": "call_2", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": [
                        {"type": "document", "document": {"id": "res-x"}},
                        {"type": "document", "document": {"id": "res-y"}}
                    ]},
                    {"role": "tool", "tool_call_id": "call_2", "content": [
                        {"type": "document", "document": {"id": "res-z"}}
                    ]}
                ]"#,
            );
            let ids = PromptRenderIds::from_messages(&msgs, &[]);
            assert_eq!(
                ids.tool_call_ids,
                vec!["call_1".to_string(), "call_2".to_string()]
            );
            assert_eq!(
                ids.document_ids,
                vec![
                    vec!["res-x".to_string(), "res-y".to_string()],
                    vec!["res-z".to_string()],
                ]
            );
        }

        #[test]
        fn top_level_docs_interleave_with_tool_calls() {
            let msgs = parse_messages(
                r#"[
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": [
                        {"type": "document", "document": {"id": "res-1"}}
                    ]}
                ]"#,
            );
            let docs = parse_documents(r#"[{"id": "doc-a"}]"#);
            let ids = PromptRenderIds::from_messages(&msgs, &docs);
            assert_eq!(ids.tool_call_ids, vec![String::new(), "call_1".to_string()]);
            assert_eq!(
                ids.document_ids,
                vec![vec!["doc-a".to_string()], vec!["res-1".to_string()]]
            );
        }

        #[test]
        fn missing_ids_default_to_empty_string() {
            let msgs = parse_messages(
                r#"[
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": [
                        {"type": "document", "document": {}}
                    ]}
                ]"#,
            );
            let ids = PromptRenderIds::from_messages(&msgs, &[]);
            assert_eq!(ids.document_ids, vec![vec![String::new()]]);
        }

        // `from_messages` is deliberately infallible: template-shape
        // errors are the renderer's problem, not the parser's. The tests
        // below pin the graceful-degradation behaviour so the parser
        // never surfaces a `TemplateValidation` back to a caller.

        #[test]
        fn tool_message_missing_tool_call_id_gets_empty_bucket() {
            let msgs = parse_messages(r#"[{"role": "tool", "content": []}]"#);
            let ids = PromptRenderIds::from_messages(&msgs, &[]);
            assert_eq!(ids.tool_call_ids, vec![String::new()]);
            assert_eq!(ids.document_ids, vec![Vec::<String>::new()]);
        }

        #[test]
        fn empty_tool_call_id_gets_empty_bucket() {
            let msgs = parse_messages(
                r#"[{"role": "chatbot", "content": [], "tool_calls": [
                    {"id": "", "name": "search", "parameters": "{}"}
                ]}]"#,
            );
            let ids = PromptRenderIds::from_messages(&msgs, &[]);
            assert_eq!(ids.tool_call_ids, vec![String::new()]);
        }

        #[test]
        fn duplicate_tool_call_id_reuses_bucket() {
            let msgs = parse_messages(
                r#"[
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]}
                ]"#,
            );
            let ids = PromptRenderIds::from_messages(&msgs, &[]);
            assert_eq!(ids.tool_call_ids, vec!["call_1".to_string()]);
        }

        #[test]
        fn tool_calls_on_non_chatbot_still_number() {
            let msgs = parse_messages(
                r#"[{"role": "user", "content": [], "tool_calls": [
                    {"id": "call_1", "name": "search", "parameters": "{}"}
                ]}]"#,
            );
            let ids = PromptRenderIds::from_messages(&msgs, &[]);
            assert_eq!(ids.tool_call_ids, vec!["call_1".to_string()]);
        }

        #[test]
        fn matches_render_cmd3_numbering() {
            // Sanity check: whatever the renderer produced (via the shared
            // helper) must line up with what `from_messages` returns
            // standalone. Round-trip through render_cmd3 to a produced
            // prompt substring to make sure the numbering is real.
            use crate::templating::{RenderCmd3Options, render_cmd3};
            let json = r#"{
                "messages": [
                    {"role": "user", "content": [{"type": "text", "text": "Hi"}]},
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": [
                        {"type": "document", "document": {"id": "res-1"}}
                    ]}
                ],
                "documents": [{"id": "doc-a"}, {"id": "doc-b"}]
            }"#;
            let value: Value = serde_json::from_str(json).unwrap();
            let opts: RenderCmd3Options = serde_path_to_error::deserialize(&value).unwrap();

            let standalone = PromptRenderIds::from_messages(&opts.messages, &opts.documents);
            // Top-level docs go to bucket 0, the tool call gets bucket 1.
            assert_eq!(standalone.tool_call_ids, vec!["", "call_1"]);
            assert_eq!(
                standalone.document_ids,
                vec![vec!["doc-a", "doc-b"], vec!["res-1"]]
            );
            // Prompt must actually render successfully with the same input.
            assert!(render_cmd3(&opts).is_ok());
        }
    }

    mod messages_to_template_validation {
        //! Renderer-side validation lives inside `messages_to_template`'s
        //! single message walk (via `validate_message_for_rendering`). We
        //! test it through the public entry point so the tests don't pin
        //! the helper's exact shape.
        use super::*;

        fn parse_messages(json: &str) -> Vec<Message> {
            let value: Value = serde_json::from_str(json).unwrap();
            serde_path_to_error::deserialize(&value).unwrap()
        }

        fn expect_validation_error(msgs: &[Message], needle: &str) {
            let err = super::super::messages_to_template(msgs, &[], &BTreeMap::new()).unwrap_err();
            let MelodyError::TemplateValidation(msg) = err else {
                panic!("expected TemplateValidation error, got {err:?}");
            };
            assert!(
                msg.contains(needle),
                "expected error message to contain {needle:?}, got {msg:?}"
            );
        }

        #[test]
        fn accepts_well_formed_history() {
            let msgs = parse_messages(
                r#"[
                    {"role": "user", "content": [{"type": "text", "text": "hi"}]},
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": []}
                ]"#,
            );
            assert!(super::super::messages_to_template(&msgs, &[], &BTreeMap::new()).is_ok());
        }

        #[test]
        fn rejects_tool_message_without_tool_call_id() {
            let msgs = parse_messages(r#"[{"role": "tool", "content": []}]"#);
            expect_validation_error(&msgs, "missing tool_call_id");
        }

        #[test]
        fn rejects_empty_tool_call_id() {
            let msgs = parse_messages(
                r#"[{"role": "chatbot", "content": [], "tool_calls": [
                    {"id": "", "name": "search", "parameters": "{}"}
                ]}]"#,
            );
            expect_validation_error(&msgs, "empty id");
        }

        #[test]
        fn rejects_duplicate_tool_call_id() {
            let msgs = parse_messages(
                r#"[
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]}
                ]"#,
            );
            expect_validation_error(&msgs, "duplicate tool call id");
        }

        #[test]
        fn rejects_tool_calls_on_non_chatbot_role() {
            let msgs = parse_messages(
                r#"[{"role": "user", "content": [], "tool_calls": [
                    {"id": "call_1", "name": "search", "parameters": "{}"}
                ]}]"#,
            );
            expect_validation_error(&msgs, "chatbot");
        }
    }
}
