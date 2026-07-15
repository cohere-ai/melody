//! Nested-xml cofl tool-call parsing for the cmd5-nested-xml template.
//!
//! Tool parameters are encoded as recursive `<cofl:value>` nodes rather than
//! flat `<cofl:tool_param>` tags:
//!
//! ```text
//! <cofl:value name="query" type="raw">echo "Hello"</cofl:value>
//! <cofl:value name="filters" type="dict">
//!   <cofl:value name="fresh" type="json">true</cofl:value>
//!   <cofl:value name="tags" type="list">
//!     <cofl:value type="raw">music</cofl:value>
//!   </cofl:value>
//! </cofl:value>
//! ```

use crate::parsing::cofl_filter::{
    decode_xml_entities, extract_attr, json_escape_string_content, split_xml_entity_holdback,
};
use crate::parsing::filter::{FilterImpl, PartialMatchResult, find_partial};
use crate::parsing::types::{FilterOutput, FilterToolCallDelta};

pub(crate) const TOOL_CALL_OPEN_START: &str = "<cofl:tool_call ";
pub(crate) const TOOL_CALL_CLOSE: &str = "</cofl:tool_call>";
const VALUE_OPEN_START: &str = "<cofl:value ";
const VALUE_CLOSE: &str = "</cofl:value>";

/// State machine modes for nested cofl value parsing.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum CoflNestedMode {
    BeforeToolCall,
    InToolCallBody,
    InContainerBody,
    InLeafBody,
}

/// Open container on the nested-value stack.
#[derive(Debug, Clone)]
pub(crate) enum CoflNestedContainer {
    ToolCallRoot { raw_object_opened: bool },
    Dict { first_entry: bool },
    List { first_entry: bool },
}

/// Per-filter state for nested cofl tool-call parsing.
#[derive(Debug, Clone)]
pub(crate) struct FilterCoflNestedAction {
    pub(crate) mode: CoflNestedMode,
    pub(crate) cur_tool_call_index: usize,
    pub(crate) containers: Vec<CoflNestedContainer>,
    pub(crate) leaf_is_raw: bool,
}

impl FilterCoflNestedAction {
    pub fn new() -> Self {
        Self {
            mode: CoflNestedMode::BeforeToolCall,
            cur_tool_call_index: 0,
            containers: Vec::new(),
            leaf_is_raw: false,
        }
    }
}

impl FilterImpl {
    pub(crate) fn parse_cofl_nested_actions(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        match self.cofl_nested_action_metadata.mode {
            CoflNestedMode::BeforeToolCall => self.handle_nested_before_tool_call(s),
            CoflNestedMode::InToolCallBody => self.handle_nested_tool_call_body(s),
            CoflNestedMode::InContainerBody => self.handle_nested_container_body(s),
            CoflNestedMode::InLeafBody => self.handle_nested_leaf_body(s),
        }
    }

    fn handle_nested_before_tool_call(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let Some(open_pos) = s.find(TOOL_CALL_OPEN_START) else {
            if s.trim_end().is_empty() {
                return (Vec::new(), s.len());
            }
            return (Vec::new(), 0);
        };

        let after_open = open_pos + TOOL_CALL_OPEN_START.len();
        let Some(close_rel) = s[after_open..].find('>') else {
            return (Vec::new(), 0);
        };
        let close_pos = after_open + close_rel;
        let attrs = &s[after_open..close_pos];

        let id = extract_attr(attrs, "id")
            .map(decode_xml_entities)
            .unwrap_or_default();
        let name = extract_attr(attrs, "name")
            .map(decode_xml_entities)
            .unwrap_or_default();

        let mut out = Vec::new();
        if self.stream_tool_actions && !id.is_empty() {
            out.push(FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: self.cofl_nested_action_metadata.cur_tool_call_index,
                    id,
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        if self.stream_tool_actions && !name.is_empty() {
            out.push(FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: self.cofl_nested_action_metadata.cur_tool_call_index,
                    name,
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        self.cofl_nested_action_metadata.containers = vec![CoflNestedContainer::ToolCallRoot {
            raw_object_opened: false,
        }];
        self.cofl_nested_action_metadata.mode = CoflNestedMode::InToolCallBody;

        let consumed = close_pos + 1;
        let (o, r) = self.parse_cofl_nested_actions(&s[consumed..]);
        out.extend(o);
        (out, consumed + r)
    }

    fn handle_nested_tool_call_body(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let close_call_pos = s.find(TOOL_CALL_CLOSE);
        let open_value_pos = s.find(VALUE_OPEN_START);

        match (close_call_pos, open_value_pos) {
            (Some(c), Some(v)) if v < c => self.handle_nested_open_value(s, v),
            (Some(c), _) => self.handle_nested_close_tool_call(s, c),
            (None, Some(v)) => self.handle_nested_open_value(s, v),
            (None, None) => (Vec::new(), 0),
        }
    }

    fn handle_nested_container_body(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let close_value_pos = s.find(VALUE_CLOSE);
        let open_value_pos = s.find(VALUE_OPEN_START);

        match (close_value_pos, open_value_pos) {
            (Some(c), Some(v)) if v < c => self.handle_nested_open_value(s, v),
            (Some(c), _) => self.handle_nested_close_container(s, c),
            (None, Some(v)) => self.handle_nested_open_value(s, v),
            (None, None) => (Vec::new(), 0),
        }
    }

    fn handle_nested_open_value(
        &mut self,
        s: &str,
        value_pos: usize,
    ) -> (Vec<FilterOutput>, usize) {
        let after_open = value_pos + VALUE_OPEN_START.len();
        let Some(close_rel) = s[after_open..].find('>') else {
            return (Vec::new(), 0);
        };
        let close_pos = after_open + close_rel;
        let attrs = &s[after_open..close_pos];
        let name = extract_attr(attrs, "name").map(decode_xml_entities);
        let value_type = extract_attr(attrs, "type").unwrap_or("raw");

        let after_tag = close_pos + 1;
        let is_empty = s[after_tag..].starts_with(VALUE_CLOSE);

        let mut out = Vec::new();

        match value_type {
            "dict" if is_empty => {
                out.extend(self.emit_nested_empty_container(name.as_deref(), "{", "}"));
            }
            "list" if is_empty => {
                out.extend(self.emit_nested_empty_container(name.as_deref(), "[", "]"));
            }
            "dict" => {
                out.extend(self.emit_nested_container_open(name.as_deref(), "{"));
                self.cofl_nested_action_metadata
                    .containers
                    .push(CoflNestedContainer::Dict { first_entry: true });
                self.cofl_nested_action_metadata.mode = CoflNestedMode::InContainerBody;
            }
            "list" => {
                out.extend(self.emit_nested_container_open(name.as_deref(), "["));
                self.cofl_nested_action_metadata
                    .containers
                    .push(CoflNestedContainer::List { first_entry: true });
                self.cofl_nested_action_metadata.mode = CoflNestedMode::InContainerBody;
            }
            "json" => {
                out.extend(self.emit_nested_leaf_open(name.as_deref(), false));
                self.cofl_nested_action_metadata.leaf_is_raw = false;
                self.cofl_nested_action_metadata.mode = CoflNestedMode::InLeafBody;
            }
            _ => {
                // `raw` and unknown types default to raw string bodies.
                out.extend(self.emit_nested_leaf_open(name.as_deref(), true));
                self.cofl_nested_action_metadata.leaf_is_raw = true;
                self.cofl_nested_action_metadata.mode = CoflNestedMode::InLeafBody;
            }
        }

        // Empty containers consume `</cofl:value>` here. Empty leaves do not —
        // they leave the close tag for `InLeafBody` so the closing quote / mode
        // reset run. Consuming the close while still in `InLeafBody` left later
        // siblings parsed as stray leaf text.
        let empty_container = is_empty && matches!(value_type, "dict" | "list");
        let consumed = if empty_container {
            after_tag + VALUE_CLOSE.len()
        } else {
            close_pos + 1
        };

        if empty_container {
            self.mark_nested_parent_entry_seen();
        }
        let (o, r) = self.parse_cofl_nested_actions(&s[consumed..]);
        out.extend(o);
        (out, consumed + r)
    }

    fn handle_nested_close_container(
        &mut self,
        s: &str,
        close_pos: usize,
    ) -> (Vec<FilterOutput>, usize) {
        let mut out = Vec::new();

        let closing = match self.cofl_nested_action_metadata.containers.last() {
            Some(CoflNestedContainer::Dict { .. }) => "}",
            Some(CoflNestedContainer::List { .. }) => "]",
            Some(CoflNestedContainer::ToolCallRoot { .. }) | None => {
                return (Vec::new(), 0);
            }
        };

        if self.stream_tool_actions && !self.stream_processed_params {
            out.push(FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: self.cofl_nested_action_metadata.cur_tool_call_index,
                    raw_param_delta: closing.to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        self.cofl_nested_action_metadata.containers.pop();
        self.mark_nested_parent_entry_seen();
        self.cofl_nested_action_metadata.mode =
            if self.cofl_nested_action_metadata.containers.is_empty()
                || matches!(
                    self.cofl_nested_action_metadata.containers.last(),
                    Some(CoflNestedContainer::ToolCallRoot { .. })
                )
            {
                CoflNestedMode::InToolCallBody
            } else {
                CoflNestedMode::InContainerBody
            };

        let consumed = close_pos + VALUE_CLOSE.len();
        let (o, r) = self.parse_cofl_nested_actions(&s[consumed..]);
        out.extend(o);
        (out, consumed + r)
    }

    fn handle_nested_close_tool_call(
        &mut self,
        s: &str,
        close_pos: usize,
    ) -> (Vec<FilterOutput>, usize) {
        let mut out = Vec::new();

        if self.stream_tool_actions && !self.stream_processed_params {
            let closing = match self.cofl_nested_action_metadata.containers.first() {
                Some(CoflNestedContainer::ToolCallRoot {
                    raw_object_opened: true,
                }) => "}",
                _ => "{}",
            };
            out.push(FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: self.cofl_nested_action_metadata.cur_tool_call_index,
                    raw_param_delta: closing.to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        self.cofl_nested_action_metadata.cur_tool_call_index += 1;
        self.cofl_nested_action_metadata.containers.clear();
        self.cofl_nested_action_metadata.mode = CoflNestedMode::BeforeToolCall;

        let consumed = close_pos + TOOL_CALL_CLOSE.len();
        let (o, r) = self.parse_cofl_nested_actions(&s[consumed..]);
        out.extend(o);
        (out, consumed + r)
    }

    fn handle_nested_leaf_body(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let stops = [VALUE_CLOSE.to_string()];
        let (value_end, close_consumed, is_final) = match find_partial(s, stops.iter()) {
            PartialMatchResult::NoMatch => (s.len(), s.len(), false),
            PartialMatchResult::Partial { idx } => (idx, idx, false),
            PartialMatchResult::Full { idx, sequence } => (idx, idx + sequence.len(), true),
        };

        let (decodable, entity_holdback) = if self.cofl_decode_xml_text {
            split_xml_entity_holdback(&s[..value_end], is_final)
        } else {
            (&s[..value_end], "")
        };
        let value_consumed = value_end - entity_holdback.len();

        let mut out = self.emit_nested_leaf_chunk(decodable, is_final);

        if is_final {
            self.mark_nested_parent_entry_seen();
            self.cofl_nested_action_metadata.mode =
                if self.cofl_nested_action_metadata.containers.len() <= 1 {
                    CoflNestedMode::InToolCallBody
                } else {
                    CoflNestedMode::InContainerBody
                };
            let (more, r) = self.parse_cofl_nested_actions(&s[close_consumed..]);
            out.extend(more);
            (out, close_consumed + r)
        } else {
            (out, value_consumed)
        }
    }

    fn emit_nested_key_prefix(&mut self, key: Option<&str>) -> String {
        let Some(container) = self.cofl_nested_action_metadata.containers.last_mut() else {
            return String::new();
        };

        match container {
            CoflNestedContainer::ToolCallRoot { raw_object_opened } => {
                let key = key.unwrap_or_default();
                if *raw_object_opened {
                    format!(r#", "{}": "#, json_escape_string_content(key))
                } else {
                    *raw_object_opened = true;
                    format!(r#"{{"{}": "#, json_escape_string_content(key))
                }
            }
            CoflNestedContainer::Dict { first_entry } => {
                let key = key.unwrap_or_default();
                if *first_entry {
                    *first_entry = false;
                    format!(r#""{}": "#, json_escape_string_content(key))
                } else {
                    format!(r#", "{}": "#, json_escape_string_content(key))
                }
            }
            CoflNestedContainer::List { first_entry } => {
                if *first_entry {
                    *first_entry = false;
                    String::new()
                } else {
                    ", ".to_string()
                }
            }
        }
    }

    fn emit_nested_empty_container(
        &mut self,
        key: Option<&str>,
        open: &str,
        close: &str,
    ) -> Vec<FilterOutput> {
        if !self.stream_tool_actions || self.stream_processed_params {
            return Vec::new();
        }

        let prefix = self.emit_nested_key_prefix(key);
        let delta = format!("{prefix}{open}{close}");
        vec![FilterOutput {
            tool_call_delta: Some(FilterToolCallDelta {
                index: self.cofl_nested_action_metadata.cur_tool_call_index,
                raw_param_delta: delta,
                ..Default::default()
            }),
            ..Default::default()
        }]
    }

    fn emit_nested_container_open(&mut self, key: Option<&str>, open: &str) -> Vec<FilterOutput> {
        if !self.stream_tool_actions || self.stream_processed_params {
            return Vec::new();
        }

        let prefix = self.emit_nested_key_prefix(key);
        vec![FilterOutput {
            tool_call_delta: Some(FilterToolCallDelta {
                index: self.cofl_nested_action_metadata.cur_tool_call_index,
                raw_param_delta: format!("{prefix}{open}"),
                ..Default::default()
            }),
            ..Default::default()
        }]
    }

    fn emit_nested_leaf_open(&mut self, key: Option<&str>, is_raw: bool) -> Vec<FilterOutput> {
        if !self.stream_tool_actions || self.stream_processed_params {
            return Vec::new();
        }

        let mut delta = self.emit_nested_key_prefix(key);
        if is_raw {
            delta.push('"');
        }
        if delta.is_empty() {
            return Vec::new();
        }
        vec![FilterOutput {
            tool_call_delta: Some(FilterToolCallDelta {
                index: self.cofl_nested_action_metadata.cur_tool_call_index,
                raw_param_delta: delta,
                ..Default::default()
            }),
            ..Default::default()
        }]
    }

    fn emit_nested_leaf_chunk(&mut self, chunk: &str, is_final: bool) -> Vec<FilterOutput> {
        if !self.stream_tool_actions || self.stream_processed_params {
            return Vec::new();
        }
        if chunk.is_empty() && !is_final {
            return Vec::new();
        }

        let decoded = if self.cofl_decode_xml_text {
            decode_xml_entities(chunk)
        } else {
            chunk.to_string()
        };

        let mut delta = if self.cofl_nested_action_metadata.leaf_is_raw {
            json_escape_string_content(&decoded)
        } else {
            decoded
        };
        if is_final && self.cofl_nested_action_metadata.leaf_is_raw {
            delta.push('"');
        } else if is_final && delta.is_empty() {
            // Empty `type="json"` body — emit null so the key stays a valid pair.
            delta.push_str("null");
        }

        if delta.is_empty() {
            return Vec::new();
        }

        vec![FilterOutput {
            tool_call_delta: Some(FilterToolCallDelta {
                index: self.cofl_nested_action_metadata.cur_tool_call_index,
                raw_param_delta: delta,
                ..Default::default()
            }),
            ..Default::default()
        }]
    }

    fn mark_nested_parent_entry_seen(&mut self) {
        if self.cofl_nested_action_metadata.containers.len() < 2 {
            return;
        }
        let parent = self.cofl_nested_action_metadata.containers.len() - 2;
        match &mut self.cofl_nested_action_metadata.containers[parent] {
            CoflNestedContainer::Dict { first_entry }
            | CoflNestedContainer::List { first_entry } => {
                *first_entry = false;
            }
            CoflNestedContainer::ToolCallRoot { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::filter::FilterImpl;

    fn fresh_nested_filter() -> FilterImpl {
        let mut filter = FilterImpl::new();
        filter.stream_tool_actions = true;
        filter.cofl_tool_action = true;
        filter.cofl_nested_xml = true;
        filter.cofl_decode_xml_text = false;
        filter
    }

    fn collect_raw(out: &[FilterOutput]) -> String {
        out.iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect()
    }

    #[test]
    fn test_parse_nested_xml_example_tool_call() {
        let mut f = fresh_nested_filter();
        let input = r#"<cofl:tool_call id="0" name="search &quot;web&quot;"><cofl:value name="query" type="raw">echo "Hello" >> foo.txt && exit</cofl:value><cofl:value name="limit" type="json">3</cofl:value><cofl:value name="float example" type="json">3.14</cofl:value><cofl:value name="filters" type="dict"><cofl:value name="fresh" type="json">true</cofl:value><cofl:value name="tags" type="list"><cofl:value type="raw">music</cofl:value><cofl:value type="raw">Sudan</cofl:value></cofl:value></cofl:value><cofl:value name="missing" type="json">null</cofl:value><cofl:value name="empty_dict" type="dict"></cofl:value><cofl:value name="empty_list" type="list"></cofl:value></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_nested_actions(input);
        assert_eq!(consumed, input.len());

        let name = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .find_map(|d| {
                if d.name.is_empty() {
                    None
                } else {
                    Some(d.name.as_str())
                }
            });
        assert_eq!(name, Some(r#"search "web""#));

        let raw = collect_raw(&out);
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        assert_eq!(parsed["query"], r#"echo "Hello" >> foo.txt && exit"#);
        assert_eq!(parsed["limit"], 3);
        assert_eq!(parsed["float example"], 3.14);
        assert_eq!(
            parsed["filters"],
            serde_json::json!({"fresh": true, "tags": ["music", "Sudan"]})
        );
        assert_eq!(parsed["missing"], serde_json::Value::Null);
        assert_eq!(parsed["empty_dict"], serde_json::json!({}));
        assert_eq!(parsed["empty_list"], serde_json::json!([]));
    }

    #[test]
    fn test_parse_nested_xml_empty_tool_call() {
        let mut f = fresh_nested_filter();
        let input = r#"<cofl:tool_call id="0" name="GetReminders"></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_nested_actions(input);
        assert_eq!(consumed, input.len());
        assert_eq!(collect_raw(&out), "{}");
    }

    #[test]
    fn test_parse_nested_xml_empty_raw_and_json_leaves() {
        let mut f = fresh_nested_filter();
        // cmd5-nested-xml emits empty strings as `<cofl:value ... type="raw"></cofl:value>`.
        let input = r#"<cofl:tool_call id="0" name="run"><cofl:value name="empty_raw" type="raw"></cofl:value><cofl:value name="next" type="raw">after</cofl:value><cofl:value name="empty_json" type="json"></cofl:value><cofl:value name="flag" type="json">true</cofl:value></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_nested_actions(input);
        assert_eq!(consumed, input.len());
        assert_eq!(
            f.cofl_nested_action_metadata.mode,
            CoflNestedMode::BeforeToolCall
        );

        let raw = collect_raw(&out);
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        assert_eq!(parsed["empty_raw"], "");
        assert_eq!(parsed["next"], "after");
        assert_eq!(parsed["empty_json"], serde_json::Value::Null);
        assert_eq!(parsed["flag"], true);
    }

    #[test]
    fn test_parse_nested_xml_extra_whitespace_in_open_tags() {
        let mut f = fresh_nested_filter();
        // Multiple spaces between tag name and attrs, and between attrs, are fine
        // because open markers are `"<cofl:tool_call "` / `"<cofl:value "` and
        // attribute extraction scans the remainder for `key="..."`.
        let input = r#"<cofl:tool_call            id="0"    name="search"><cofl:value     name="query"   type="raw">hello</cofl:value><cofl:value  name="tags"  type="list"><cofl:value  type="raw">a</cofl:value></cofl:value></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_nested_actions(input);
        assert_eq!(consumed, input.len());

        let name = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .find_map(|d| (!d.name.is_empty()).then_some(d.name.as_str()));
        assert_eq!(name, Some("search"));

        let raw = collect_raw(&out);
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        assert_eq!(parsed["query"], "hello");
        assert_eq!(parsed["tags"], serde_json::json!(["a"]));
    }

    #[test]
    fn test_parse_nested_xml_streaming_in_pieces() {
        let full = r#"<cofl:tool_call id="0" name="search"><cofl:value name="query" type="raw">hello</cofl:value></cofl:tool_call>"#;
        let mut combined = String::new();
        let mut f = fresh_nested_filter();
        let mut buf = String::new();
        for c in full.chars() {
            buf.push(c);
            let (out, consumed) = f.parse_cofl_nested_actions(&buf);
            combined.push_str(&collect_raw(&out));
            buf.drain(..consumed);
        }
        assert!(buf.is_empty());
        assert_eq!(combined, r#"{"query": "hello"}"#);
    }
}
