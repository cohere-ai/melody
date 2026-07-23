//! Nested-xml cofl tool-call parsing for the default cmd5 template.
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
use crate::parsing::filter::{FilterImpl, PartialMatchResult};
use crate::parsing::types::{FilterOutput, FilterToolCallDelta, FilterToolParameter};
use regex::Regex;
use std::sync::LazyLock;

/// Tag matchers mirror `datatools.renderer.parse_nested_xml`: whitespace is
/// allowed after `<` before an open tag name, after `/` in a close tag, and
/// before `>`. Close tags must start with `</` — a space between `<` and `/`
/// is not accepted. Leading `\s*` on the body/close patterns also consumes
/// indentation between sibling tags.
static TOOL_CALL_OPEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s*<\s*cofl:tool_call\s+([^>]*)>").expect("invalid tool_call open regex")
});
static VALUE_OPEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s*<\s*cofl:value\s+([^>]*)>").expect("invalid value open regex")
});
static TOOL_CALL_CLOSE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s*</\s*cofl:tool_call\s*>").expect("invalid tool_call close regex")
});
static VALUE_CLOSE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*</\s*cofl:value\s*>").expect("invalid value close regex"));
/// Leaf bodies must not treat trailing value whitespace as part of the close tag.
static VALUE_CLOSE_IN_LEAF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"</\s*cofl:value\s*>").expect("invalid leaf value close regex"));

/// Return true when `suffix` (starting at `<`) is an *incomplete* close tag
/// (`</\s*{local_name}\s*` with no `>` yet). Used to hold back while streaming;
/// complete closes are matched by `VALUE_CLOSE_IN_LEAF_RE` instead.
/// Requires `/` immediately after `<` (no `< /...>`).
fn is_close_tag_prefix(suffix: &str, local_name: &str) -> bool {
    let bytes = suffix.as_bytes();
    if bytes.first().copied() != Some(b'<') {
        return false;
    }
    if bytes.len() == 1 {
        return true;
    }
    if bytes[1] != b'/' {
        return false;
    }
    let mut i = 2;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i == bytes.len() {
        return true;
    }
    let remaining = &bytes[i..];
    let name = local_name.as_bytes();
    if remaining.len() < name.len() {
        return name.starts_with(remaining);
    }
    if !remaining.starts_with(name) {
        return false;
    }
    // Name is complete: hold back only while `>` has not arrived yet.
    remaining[name.len()..]
        .iter()
        .all(|&b| b.is_ascii_whitespace())
}

fn find_leaf_value_close(s: &str) -> PartialMatchResult {
    if let Some(mat) = VALUE_CLOSE_IN_LEAF_RE.find(s) {
        return PartialMatchResult::Full {
            idx: mat.start(),
            sequence: mat.as_str().to_string(),
        };
    }
    if let Some(lt) = s.rfind('<') {
        let suffix = &s[lt..];
        if is_close_tag_prefix(suffix, "cofl:value") {
            return PartialMatchResult::Partial { idx: lt };
        }
    }
    PartialMatchResult::NoMatch
}

/// State machine modes for nested cofl value parsing.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum CoflNestedMode {
    BeforeToolCall,
    InToolCallBody,
    InContainerBody,
    /// Inside a `type="raw"` / `type="json"` leaf. `leaf_is_raw` is only
    /// meaningful here, so it lives on the variant rather than the struct.
    ///
    /// `saw_body` tracks whether any body bytes have already been emitted for
    /// this leaf. Needed so a later close-only chunk (`</cofl:value>` arriving
    /// alone after the value streamed earlier) does not treat the leaf as
    /// empty and append a spurious `null` for `type="json"`.
    InLeafBody {
        leaf_is_raw: bool,
        saw_body: bool,
    },
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
    /// Name of the top-level parameter currently being streamed when
    /// `stream_processed_params` is enabled.
    pub(crate) cur_param_name: String,
}

impl FilterCoflNestedAction {
    pub fn new() -> Self {
        Self {
            mode: CoflNestedMode::BeforeToolCall,
            cur_tool_call_index: 0,
            containers: Vec::new(),
            cur_param_name: String::new(),
        }
    }
}

impl FilterImpl {
    pub(crate) fn parse_cofl_nested_actions(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        match self.cofl_nested_action_metadata.mode {
            CoflNestedMode::BeforeToolCall => self.handle_nested_before_tool_call(s),
            CoflNestedMode::InToolCallBody => self.handle_nested_tool_call_body(s),
            CoflNestedMode::InContainerBody => self.handle_nested_container_body(s),
            CoflNestedMode::InLeafBody { .. } => self.handle_nested_leaf_body(s),
        }
    }

    fn handle_nested_before_tool_call(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let Some(caps) = TOOL_CALL_OPEN_RE.captures(s) else {
            if s.trim_end().is_empty() {
                return (Vec::new(), s.len());
            }
            return (Vec::new(), 0);
        };
        let full = caps.get(0).expect("full match");
        let attrs = caps.get(1).map_or("", |m| m.as_str());

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

        let consumed = full.end();
        let (o, r) = self.parse_cofl_nested_actions(&s[consumed..]);
        out.extend(o);
        (out, consumed + r)
    }

    fn handle_nested_tool_call_body(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let close_mat = TOOL_CALL_CLOSE_RE.find(s);
        let open_caps = VALUE_OPEN_RE.captures(s);
        let open_start = open_caps.as_ref().and_then(|c| c.get(0)).map(|m| m.start());

        match (close_mat, open_caps, open_start) {
            (_, Some(caps), Some(v)) if close_mat.is_none_or(|c| v < c.start()) => {
                self.handle_nested_open_value(s, &caps)
            }
            (Some(c), _, _) => self.handle_nested_close_tool_call(s, c.end()),
            _ => (Vec::new(), 0),
        }
    }

    fn handle_nested_container_body(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let close_mat = VALUE_CLOSE_RE.find(s);
        let open_caps = VALUE_OPEN_RE.captures(s);
        let open_start = open_caps.as_ref().and_then(|c| c.get(0)).map(|m| m.start());

        match (close_mat, open_caps, open_start) {
            (_, Some(caps), Some(v)) if close_mat.is_none_or(|c| v < c.start()) => {
                self.handle_nested_open_value(s, &caps)
            }
            (Some(c), _, _) => self.handle_nested_close_container(s, c.end()),
            _ => (Vec::new(), 0),
        }
    }

    fn handle_nested_open_value(
        &mut self,
        s: &str,
        caps: &regex::Captures<'_>,
    ) -> (Vec<FilterOutput>, usize) {
        let full = caps.get(0).expect("full match");
        let attrs = caps.get(1).map_or("", |m| m.as_str());
        let name = extract_attr(attrs, "name").map(decode_xml_entities);
        let value_type = extract_attr(attrs, "type").unwrap_or("raw");

        let mut out = Vec::new();

        match value_type {
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
                self.cofl_nested_action_metadata.mode = CoflNestedMode::InLeafBody {
                    leaf_is_raw: false,
                    saw_body: false,
                };
            }
            _ => {
                // `raw` and unknown types default to raw string bodies.
                out.extend(self.emit_nested_leaf_open(name.as_deref(), true));
                self.cofl_nested_action_metadata.mode = CoflNestedMode::InLeafBody {
                    leaf_is_raw: true,
                    saw_body: false,
                };
            }
        }

        // Leave any immediate `</cofl:value>` for the new mode so empty
        // containers close via `InContainerBody` and empty leaves still run
        // closing-quote / `null` handling in `InLeafBody`.
        let consumed = full.end();
        let (o, r) = self.parse_cofl_nested_actions(&s[consumed..]);
        out.extend(o);
        (out, consumed + r)
    }

    fn handle_nested_close_container(
        &mut self,
        s: &str,
        consumed_end: usize,
    ) -> (Vec<FilterOutput>, usize) {
        let mut out = Vec::new();

        let closing = match self.cofl_nested_action_metadata.containers.last() {
            Some(CoflNestedContainer::Dict { .. }) => "}",
            Some(CoflNestedContainer::List { .. }) => "]",
            Some(CoflNestedContainer::ToolCallRoot { .. }) | None => {
                return (Vec::new(), 0);
            }
        };

        out.extend(self.emit_nested_stream_delta(closing.to_string()));

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

        let (o, r) = self.parse_cofl_nested_actions(&s[consumed_end..]);
        out.extend(o);
        (out, consumed_end + r)
    }

    fn handle_nested_close_tool_call(
        &mut self,
        s: &str,
        consumed_end: usize,
    ) -> (Vec<FilterOutput>, usize) {
        let mut out = Vec::new();

        // Only the raw stream needs a closing brace. The processed-params
        // stream is naturally closed by the absence of further `param_delta`s
        // for this tool call's index.
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
        self.cofl_nested_action_metadata.cur_param_name.clear();
        self.cofl_nested_action_metadata.mode = CoflNestedMode::BeforeToolCall;

        let (o, r) = self.parse_cofl_nested_actions(&s[consumed_end..]);
        out.extend(o);
        (out, consumed_end + r)
    }

    fn handle_nested_leaf_body(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let (value_end, close_consumed, is_final) = match find_leaf_value_close(s) {
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

    fn nested_at_tool_call_root(&self) -> bool {
        matches!(
            self.cofl_nested_action_metadata.containers.last(),
            Some(CoflNestedContainer::ToolCallRoot { .. })
        )
    }

    /// Emit either a `param_delta` or `raw_param_delta` chunk, mirroring the
    /// flat cofl parser: only one of the two streams is populated.
    fn emit_nested_stream_delta(&self, delta: String) -> Vec<FilterOutput> {
        if !self.stream_tool_actions || delta.is_empty() {
            return Vec::new();
        }

        let tool_call_delta = if self.stream_processed_params {
            FilterToolCallDelta {
                index: self.cofl_nested_action_metadata.cur_tool_call_index,
                param_delta: Some(FilterToolParameter {
                    name: self.cofl_nested_action_metadata.cur_param_name.clone(),
                    value_delta: delta,
                }),
                ..Default::default()
            }
        } else {
            FilterToolCallDelta {
                index: self.cofl_nested_action_metadata.cur_tool_call_index,
                raw_param_delta: delta,
                ..Default::default()
            }
        };

        vec![FilterOutput {
            tool_call_delta: Some(tool_call_delta),
            ..Default::default()
        }]
    }

    /// Begin a nested entry under `key`, announcing a top-level `param_delta`
    /// when `stream_processed_params` is set and returning any structural
    /// prefix that should be prepended to the entry's value.
    fn begin_nested_entry(&mut self, key: Option<&str>) -> (Vec<FilterOutput>, String) {
        let at_root = self.nested_at_tool_call_root();
        let prefix = self.emit_nested_key_prefix(key);

        if !self.stream_tool_actions {
            return (Vec::new(), String::new());
        }

        if self.stream_processed_params && at_root {
            // Top-level params stream as structured name/value pairs. The
            // synthetic object wrappers (`{"key": ` / `, "key": `) belong only
            // to the raw JSON accumulator and are discarded here.
            let name = key.unwrap_or_default().to_string();
            self.cofl_nested_action_metadata
                .cur_param_name
                .clone_from(&name);
            (
                vec![FilterOutput {
                    tool_call_delta: Some(FilterToolCallDelta {
                        index: self.cofl_nested_action_metadata.cur_tool_call_index,
                        param_delta: Some(FilterToolParameter {
                            name,
                            value_delta: String::new(),
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                String::new(),
            )
        } else {
            (Vec::new(), prefix)
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

    fn emit_nested_container_open(&mut self, key: Option<&str>, open: &str) -> Vec<FilterOutput> {
        let (mut out, prefix) = self.begin_nested_entry(key);
        out.extend(self.emit_nested_stream_delta(format!("{prefix}{open}")));
        out
    }

    fn emit_nested_leaf_open(&mut self, key: Option<&str>, is_raw: bool) -> Vec<FilterOutput> {
        let (mut out, mut delta) = self.begin_nested_entry(key);
        if is_raw {
            delta.push('"');
        }
        out.extend(self.emit_nested_stream_delta(delta));
        out
    }

    fn emit_nested_leaf_chunk(&mut self, chunk: &str, is_final: bool) -> Vec<FilterOutput> {
        if !self.stream_tool_actions {
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

        let (leaf_is_raw, saw_body) = match &mut self.cofl_nested_action_metadata.mode {
            CoflNestedMode::InLeafBody {
                leaf_is_raw,
                saw_body,
            } => (*leaf_is_raw, saw_body),
            _ => return Vec::new(),
        };

        let mut delta = if leaf_is_raw {
            json_escape_string_content(&decoded)
        } else {
            decoded
        };

        // Record body before finalize side-effects so a same-buffer value+close
        // (`0.1</cofl:value>`) is not mistaken for an empty json leaf.
        if !delta.is_empty() {
            *saw_body = true;
        }

        if is_final && leaf_is_raw {
            delta.push('"');
        } else if is_final && !leaf_is_raw && !*saw_body {
            // Empty `type="json"` body — emit null so the key stays a valid pair.
            delta.push_str("null");
        }

        self.emit_nested_stream_delta(delta)
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

    fn fresh_nested_processed_filter() -> FilterImpl {
        let mut filter = fresh_nested_filter();
        filter.stream_processed_params = true;
        filter
    }

    fn collect_raw(out: &[FilterOutput]) -> String {
        out.iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect()
    }

    fn collect_processed(out: &[FilterOutput]) -> Vec<(String, String)> {
        out.iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .filter_map(|d| d.param_delta.as_ref())
            .map(|p| (p.name.clone(), p.value_delta.clone()))
            .collect()
    }

    fn aggregate_processed_values(out: &[FilterOutput]) -> Vec<(String, String)> {
        let mut aggregated: Vec<(String, String)> = Vec::new();
        for (name, value) in collect_processed(out) {
            if let Some((last_name, last_value)) = aggregated.last_mut() {
                if *last_name == name {
                    last_value.push_str(&value);
                    continue;
                }
            }
            aggregated.push((name, value));
        }
        aggregated
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
        // cmd5 emits empty strings as `<cofl:value ... type="raw"></cofl:value>`.
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
        // Multiple spaces between tag name and attrs, and between attrs, are fine.
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

    /// Matches datatools `test_parse_nested_xml_parses_nested_parameters_with_permissive_whitespace`:
    /// spaces after `<` / before `>`, spaces inside closing tags, and indented multiline markup.
    #[test]
    fn test_parse_nested_xml_permissive_whitespace_like_datatools() {
        let mut f = fresh_nested_filter();
        let input = r#"
        < cofl:tool_call   id="tc-1"   name="search &quot;web&quot;" >
            < cofl:value   name="query"   type="raw" >weather in Paris</ cofl:value >
            <cofl:value name="filters" type="dict">
                < cofl:value   name="fresh"   type="json" > true </ cofl:value >
                < cofl:value name="tags" type="list">
                    <cofl:value type="raw">news</cofl:value>
                    < cofl:value type="json" > 3 </ cofl:value >
                </ cofl:value >
            </cofl:value>
        </ cofl:tool_call >
    "#;
        let (out, consumed) = f.parse_cofl_nested_actions(input);
        assert_eq!(consumed, input.len());

        let id = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .find_map(|d| (!d.id.is_empty()).then_some(d.id.as_str()));
        let name = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .find_map(|d| (!d.name.is_empty()).then_some(d.name.as_str()));
        assert_eq!(id, Some("tc-1"));
        assert_eq!(name, Some("search \"web\""));

        let raw = collect_raw(&out);
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        assert_eq!(parsed["query"], "weather in Paris");
        assert_eq!(
            parsed["filters"],
            serde_json::json!({"fresh": true, "tags": ["news", 3]})
        );
    }

    #[test]
    fn test_close_tag_rejects_space_between_lt_and_slash() {
        // Full close matchers require `</` — `< /cofl:value>` is not a close tag.
        assert!(!VALUE_CLOSE_RE.is_match(r#"< /cofl:value>"#));
        assert!(!TOOL_CALL_CLOSE_RE.is_match(r#"< /cofl:tool_call>"#));
        assert!(!VALUE_CLOSE_IN_LEAF_RE.is_match(r#"hello< /cofl:value>"#));

        // Streaming holdback must match: do not treat `< /` as a partial close.
        assert!(!is_close_tag_prefix("< /", "cofl:value"));
        assert!(!is_close_tag_prefix("< /cofl:value>", "cofl:value"));
        assert!(is_close_tag_prefix("</", "cofl:value"));
        assert!(is_close_tag_prefix("</ cofl:value", "cofl:value"));
        assert!(is_close_tag_prefix("</ cofl:value ", "cofl:value"));
        // Complete closes (including a trailing `>`) are the regex's job, not holdback.
        assert!(!is_close_tag_prefix("</ cofl:value >", "cofl:value"));
        assert!(!is_close_tag_prefix("</cofl:value>", "cofl:value"));
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

    /// Streaming a non-empty `type="json"` leaf must not append a trailing
    /// `null` when the closing tag arrives in a later chunk after the value
    /// body has already been emitted.
    #[test]
    fn test_parse_nested_xml_streaming_json_leaf_no_spurious_null() {
        let full = r#"<cofl:tool_call id="0" name="terminal_use"><cofl:value name="commands" type="list"><cofl:value type="dict"><cofl:value name="keystrokes" type="raw">which qemu-system-x86_64
</cofl:value><cofl:value name="wait" type="json">0.1</cofl:value></cofl:value></cofl:value></cofl:tool_call>"#;
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
        let parsed: serde_json::Value = serde_json::from_str(&combined).expect("valid JSON");
        assert_eq!(
            parsed,
            serde_json::json!({
                "commands": [{"keystrokes": "which qemu-system-x86_64\n", "wait": 0.1}]
            })
        );
        assert!(
            !combined.contains("0.1null"),
            "spurious null after streamed json leaf: {combined}"
        );
    }

    /// Empty `type="json"></cofl:value>` must still emit `null` when streamed
    /// char-by-char (close tag alone after an empty body).
    #[test]
    fn test_parse_nested_xml_streaming_empty_json_still_null() {
        let full = r#"<cofl:tool_call id="0" name="run"><cofl:value name="empty_json" type="json"></cofl:value></cofl:tool_call>"#;
        let mut combined = String::new();
        let mut f = fresh_nested_filter();
        let mut buf = String::new();
        for c in full.chars() {
            buf.push(c);
            let (out, consumed) = f.parse_cofl_nested_actions(&buf);
            combined.push_str(&collect_raw(&out));
            buf.drain(..consumed);
        }
        assert_eq!(combined, r#"{"empty_json": null}"#);
    }

    #[test]
    fn test_parse_nested_xml_processed_params_simple() {
        let mut f = fresh_nested_processed_filter();
        let input = r#"<cofl:tool_call id="0" name="search"><cofl:value name="query" type="raw">hello</cofl:value><cofl:value name="limit" type="json">3</cofl:value></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_nested_actions(input);
        assert_eq!(consumed, input.len());

        assert!(
            collect_raw(&out).is_empty(),
            "processed mode must not emit raw_param_delta"
        );

        let params = aggregate_processed_values(&out);
        assert_eq!(
            params,
            vec![
                ("query".to_string(), "\"hello\"".to_string()),
                ("limit".to_string(), "3".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_nested_xml_processed_params_nested_containers() {
        let mut f = fresh_nested_processed_filter();
        let input = r#"<cofl:tool_call id="0" name="search"><cofl:value name="filters" type="dict"><cofl:value name="fresh" type="json">true</cofl:value><cofl:value name="tags" type="list"><cofl:value type="raw">music</cofl:value><cofl:value type="raw">Sudan</cofl:value></cofl:value></cofl:value><cofl:value name="empty_dict" type="dict"></cofl:value><cofl:value name="empty_list" type="list"></cofl:value></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_nested_actions(input);
        assert_eq!(consumed, input.len());

        assert!(collect_raw(&out).is_empty());

        let params = aggregate_processed_values(&out);
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].0, "filters");
        let filters: serde_json::Value =
            serde_json::from_str(&params[0].1).expect("filters value is valid JSON");
        assert_eq!(
            filters,
            serde_json::json!({"fresh": true, "tags": ["music", "Sudan"]})
        );
        assert_eq!(params[1], ("empty_dict".to_string(), "{}".to_string()));
        assert_eq!(params[2], ("empty_list".to_string(), "[]".to_string()));
    }

    #[test]
    fn test_parse_nested_xml_processed_params_empty_tool_call() {
        let mut f = fresh_nested_processed_filter();
        let input = r#"<cofl:tool_call id="0" name="GetReminders"></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_nested_actions(input);
        assert_eq!(consumed, input.len());

        // Empty tool calls emit neither raw nor processed argument chunks —
        // mirroring flat cofl, which only synthesizes `{}` in raw mode.
        assert!(collect_raw(&out).is_empty());
        assert!(collect_processed(&out).is_empty());

        let id = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .find_map(|d| (!d.id.is_empty()).then_some(d.id.as_str()));
        let name = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .find_map(|d| (!d.name.is_empty()).then_some(d.name.as_str()));
        assert_eq!(id, Some("0"));
        assert_eq!(name, Some("GetReminders"));
    }
}
