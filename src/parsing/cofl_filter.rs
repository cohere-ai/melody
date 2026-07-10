//! Tool call parsing for the cofl-tagged format used by cmd5.
//!
//! Unlike the JSON-based action parser (see [`super::action_filter`]), this
//! module parses an XML-like syntax produced by the cmd5 template:
//!
//! ```text
//! <cofl:tool_call id="0" name="search">
//!   <cofl:tool_param name="query" string="true">hello</cofl:tool_param>
//!   <cofl:tool_param name="limit" string="false">3</cofl:tool_param>
//! </cofl:tool_call>
//! ```
//!
//! The outer `<cofl:tool_calls>` / `</cofl:tool_calls>` wrappers are handled
//! by the surrounding [`FilterImpl`] state machine via the special-token map
//! (they transition into / out of [`FilterMode::ToolAction`]); this parser
//! owns the inner `<cofl:tool_call>` / `<cofl:tool_param>` sequence.
//!
//! Each `<cofl:tool_param>` carries a `string` attribute:
//! - `string="true"`  -> the body is a raw string and must be JSON-encoded
//!   (wrapped in quotes, with special characters escaped) when emitted as a
//!   tool-call argument.
//! - `string="false"` -> the body is already valid JSON (e.g. a number,
//!   boolean, object or array literal) and is emitted verbatim.
//!
//! Attribute values and parameter bodies are XML-entity escaped in model
//! output (`&lt;`, `&gt;`, `&amp;`, and `&quot;` in attributes). This parser
//! decodes those entities before emitting tool-call arguments.

use crate::parsing::filter::{FilterImpl, PartialMatchResult, find_partial};
use crate::parsing::types::{FilterOutput, FilterToolCallDelta, FilterToolParameter};

/// Opening sentinel for a tool-call tag.  Includes the trailing space so we
/// do not accidentally match the outer `<cofl:tool_calls>` wrapper, which is
/// handled by the special-token map.
const TOOL_CALL_OPEN_START: &str = "<cofl:tool_call ";
const TOOL_CALL_CLOSE: &str = "</cofl:tool_call>";
const TOOL_PARAM_OPEN_START: &str = "<cofl:tool_param ";
const TOOL_PARAM_CLOSE: &str = "</cofl:tool_param>";

/// State machine modes for parsing cofl tool-call tags.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum CoflMode {
    /// Waiting for the next `<cofl:tool_call ...>` opening tag (initial
    /// state, and the state after each `</cofl:tool_call>`).
    BeforeToolCall,
    /// Inside a tool-call body, waiting for either a
    /// `<cofl:tool_param ...>` opening tag or the `</cofl:tool_call>`
    /// terminator.
    InToolCallBody,
    /// Inside the body of a `<cofl:tool_param>` tag, streaming the value
    /// until `</cofl:tool_param>` is found.
    InToolParamValue,
}

/// Per-filter state needed to incrementally parse a cofl tool-call stream.
#[derive(Debug, Clone)]
pub(crate) struct FilterCoflAction {
    pub mode: CoflMode,
    /// Index of the tool call currently being parsed (0-based).
    pub cur_tool_call_index: usize,
    /// Name of the parameter currently being parsed.
    pub cur_param_name: String,
    /// Whether the current parameter's `string="true"` attribute is set, in
    /// which case the body needs JSON string encoding when emitted.
    pub cur_param_is_string: bool,
    /// Whether we have already opened the JSON object (`{`) for the current
    /// tool call's `raw_param_delta`.
    pub raw_object_opened: bool,
}

impl FilterCoflAction {
    pub fn new() -> Self {
        Self {
            mode: CoflMode::BeforeToolCall,
            cur_tool_call_index: 0,
            cur_param_name: String::new(),
            cur_param_is_string: false,
            raw_object_opened: false,
        }
    }
}

/// XML entities emitted by the cmd5 template's `xml_text` / `xml_attr` macros.
const XML_ENTITIES: &[&str] = &["&amp;", "&lt;", "&gt;", "&quot;", "&apos;"];

/// Extract the value of an `key="..."` attribute from inside an opening tag.
///
/// Returns `None` if the attribute is missing or its value is unterminated.
/// Attribute values use `&quot;` rather than literal `"`, so a naive scan is
/// safe on the wire format.
fn extract_attr<'a>(tag_inner: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=\"");
    let start = tag_inner.find(&needle)?;
    let value_start = start + needle.len();
    let rest = &tag_inner[value_start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Decode XML entities in a complete string (attribute values and flushed
/// parameter bodies). `&amp;` is decoded last so sequences like `&amp;lt;`
/// round-trip correctly.
fn decode_xml_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Split `s` into `(decodable_prefix, holdback_suffix)` for incremental
/// XML entity decoding while streaming parameter bodies.
///
/// When a chunk ends mid-entity (e.g. `"if (a &l"` then `"t; b)"`), the
/// holdback suffix is left in the caller's buffer (via a reduced `consumed`
/// count) rather than decoded prematurely. When `flush` is true (final chunk
/// of a param), everything is decoded with no holdback.
fn split_xml_entity_holdback(s: &str, flush: bool) -> (&str, &str) {
    if flush || s.is_empty() {
        return (s, "");
    }
    // Only the last `&` can be incomplete; earlier entities are complete.
    let Some(amp_pos) = s.rfind('&') else {
        return (s, "");
    };
    let tail = &s[amp_pos..];
    // A semicolon means the entity is complete (e.g. `&lt;`), so decode now.
    if tail.contains(';') {
        return (s, "");
    }
    // Hold back if `tail` could still grow into a known entity on the next
    // chunk: a lone `&`, or a prefix like `&l` / `&lt`. Complete entities
    // (which all contain `;`) are handled above. Unknown tails like `&foo`
    // are decoded as-is.
    let hold = tail == "&" || XML_ENTITIES.iter().any(|ent| ent.starts_with(tail));
    if hold { (&s[..amp_pos], tail) } else { (s, "") }
}

/// JSON-escape the body of a string parameter so it can be emitted between
/// surrounding `"` quotes as a valid JSON string literal.
fn json_escape_string_content(s: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            // Remaining ASCII control characters (U+0000..U+001F) without a
            // dedicated short escape above. JSON forbids raw control chars in
            // string literals, so emit them as `\uXXXX` (lowercase, zero
            // padded to 4 hex digits). `write!` into a `String` is infallible,
            // so the `Result` is discarded.
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

impl FilterImpl {
    /// Entry point for parsing a chunk of cofl-tagged tool-call text.
    ///
    /// Mirrors the contract of [`Self::parse_actions`]: returns the outputs
    /// produced by the chunk and the number of bytes that may be drained
    /// from the input buffer. A return of `(_, 0)` means we need more bytes
    /// to make progress (e.g. waiting for the close of an opening tag).
    pub(crate) fn parse_cofl_actions(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        if s.is_empty() {
            return (Vec::new(), 0);
        }

        match self.cofl_action_metadata.mode {
            CoflMode::BeforeToolCall => self.handle_cofl_before_tool_call(s),
            CoflMode::InToolCallBody => self.handle_cofl_in_tool_call_body(s),
            CoflMode::InToolParamValue => self.handle_cofl_in_tool_param_value(s),
        }
    }

    fn handle_cofl_before_tool_call(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let Some(open_pos) = s.find(TOOL_CALL_OPEN_START) else {
            // No opening tag yet. Discard whitespace-only chatter between
            // tool calls but otherwise wait for more bytes.
            if s.trim_end().is_empty() {
                return (Vec::new(), s.len());
            }
            return (Vec::new(), 0);
        };

        let after_open = open_pos + TOOL_CALL_OPEN_START.len();
        let Some(close_rel) = s[after_open..].find('>') else {
            // Opening tag is incomplete; wait for more bytes.
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
                    index: self.cofl_action_metadata.cur_tool_call_index,
                    id,
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        if self.stream_tool_actions && !name.is_empty() {
            out.push(FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: self.cofl_action_metadata.cur_tool_call_index,
                    name,
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        self.cofl_action_metadata.raw_object_opened = false;
        self.cofl_action_metadata.mode = CoflMode::InToolCallBody;

        let consumed = close_pos + 1;
        let (o, r) = self.parse_cofl_actions(&s[consumed..]);
        out.extend(o);
        (out, consumed + r)
    }

    fn handle_cofl_in_tool_call_body(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        let close_call_pos = s.find(TOOL_CALL_CLOSE);
        let open_param_pos = s.find(TOOL_PARAM_OPEN_START);

        // Dispatch to whichever marker appears first in the buffer. If only
        // one is present, take that one. Otherwise wait for more bytes.
        match (close_call_pos, open_param_pos) {
            (Some(c), Some(p)) => {
                if p < c {
                    self.handle_cofl_open_param(s, p)
                } else {
                    self.handle_cofl_close_tool_call(s, c)
                }
            }
            (Some(c), None) => self.handle_cofl_close_tool_call(s, c),
            (None, Some(p)) => self.handle_cofl_open_param(s, p),
            (None, None) => (Vec::new(), 0),
        }
    }

    fn handle_cofl_open_param(&mut self, s: &str, param_pos: usize) -> (Vec<FilterOutput>, usize) {
        let after_open = param_pos + TOOL_PARAM_OPEN_START.len();
        let Some(close_rel) = s[after_open..].find('>') else {
            // Opening tag is incomplete; wait for more bytes.
            return (Vec::new(), 0);
        };
        let close_pos = after_open + close_rel;
        let attrs = &s[after_open..close_pos];

        let name = extract_attr(attrs, "name")
            .map(decode_xml_entities)
            .unwrap_or_default();
        // Default to `true` so a missing `string` attribute is treated as a
        // raw string (the conservative choice).
        let is_string = extract_attr(attrs, "string").is_none_or(|v| v != "false");

        let mut out = Vec::new();

        if self.stream_tool_actions {
            // Mirror the JSON action parser: emit either processed
            // `param_delta`s OR a synthesized `raw_param_delta` stream,
            // never both. The choice is driven by `stream_processed_params`.
            if self.stream_processed_params {
                // Announce the new parameter with an empty value, matching
                // action_filter::send_param_name_chunk.
                out.push(FilterOutput {
                    tool_call_delta: Some(FilterToolCallDelta {
                        index: self.cofl_action_metadata.cur_tool_call_index,
                        param_delta: Some(FilterToolParameter {
                            name: name.clone(),
                            value_delta: String::new(),
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
                // For string-valued params, emit the opening `"` eagerly so
                // the value-streaming code path can simply append escaped
                // chunks without tracking quote state.
                if is_string {
                    out.push(FilterOutput {
                        tool_call_delta: Some(FilterToolCallDelta {
                            index: self.cofl_action_metadata.cur_tool_call_index,
                            param_delta: Some(FilterToolParameter {
                                name: name.clone(),
                                value_delta: "\"".to_string(),
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    });
                }
            } else {
                // Append to the per-tool-call raw_param_delta accumulator,
                // synthesizing a JSON object as we go.
                let raw_prefix = if self.cofl_action_metadata.raw_object_opened {
                    format!(", \"{}\": ", json_escape_string_content(&name))
                } else {
                    self.cofl_action_metadata.raw_object_opened = true;
                    format!("{{\"{}\": ", json_escape_string_content(&name))
                };
                out.push(FilterOutput {
                    tool_call_delta: Some(FilterToolCallDelta {
                        index: self.cofl_action_metadata.cur_tool_call_index,
                        raw_param_delta: raw_prefix,
                        ..Default::default()
                    }),
                    ..Default::default()
                });
                if is_string {
                    out.push(FilterOutput {
                        tool_call_delta: Some(FilterToolCallDelta {
                            index: self.cofl_action_metadata.cur_tool_call_index,
                            raw_param_delta: "\"".to_string(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    });
                }
            }
        }

        self.cofl_action_metadata.cur_param_name = name;
        self.cofl_action_metadata.cur_param_is_string = is_string;
        self.cofl_action_metadata.mode = CoflMode::InToolParamValue;

        let consumed = close_pos + 1;
        let (o, r) = self.parse_cofl_actions(&s[consumed..]);
        out.extend(o);
        (out, consumed + r)
    }

    fn handle_cofl_close_tool_call(
        &mut self,
        s: &str,
        close_pos: usize,
    ) -> (Vec<FilterOutput>, usize) {
        let mut out = Vec::new();

        // Only the raw stream needs a closing brace. The processed-params
        // stream is naturally closed by the absence of further `param_delta`s
        // for this tool call's index.
        if self.stream_tool_actions && !self.stream_processed_params {
            // Empty-parameter tool calls still need a `{}` so downstream
            // consumers see well-formed JSON.
            let closing = if self.cofl_action_metadata.raw_object_opened {
                "}"
            } else {
                "{}"
            };
            out.push(FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: self.cofl_action_metadata.cur_tool_call_index,
                    raw_param_delta: closing.to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        self.cofl_action_metadata.cur_tool_call_index += 1;
        self.cofl_action_metadata.raw_object_opened = false;
        self.cofl_action_metadata.cur_param_name.clear();
        self.cofl_action_metadata.mode = CoflMode::BeforeToolCall;

        let consumed = close_pos + TOOL_CALL_CLOSE.len();
        let (o, r) = self.parse_cofl_actions(&s[consumed..]);
        out.extend(o);
        (out, consumed + r)
    }

    fn handle_cofl_in_tool_param_value(&mut self, s: &str) -> (Vec<FilterOutput>, usize) {
        // Use `find_partial` so a tail like `</cofl:tool_p` is treated as an
        // incomplete close tag (we emit everything before it but leave the
        // partial in the caller's buffer for the next call). Without this,
        // those bytes would leak into the streamed value.
        let stops = [TOOL_PARAM_CLOSE.to_string()];
        let (value_end, close_consumed, is_final) = match find_partial(s, stops.iter()) {
            PartialMatchResult::NoMatch => (s.len(), s.len(), false),
            PartialMatchResult::Partial { idx } => (idx, idx, false),
            PartialMatchResult::Full { idx, sequence } => (idx, idx + sequence.len(), true),
        };

        // Also hold back a trailing partial XML entity (e.g. `&l` of `&lt;`)
        // in the caller's buffer, same as partial close tags above.
        let (decodable, entity_holdback) = if self.cofl_decode_xml_text {
            split_xml_entity_holdback(&s[..value_end], is_final)
        } else {
            (&s[..value_end], "")
        };
        let value_consumed = value_end - entity_holdback.len();

        let mut out = self.emit_cofl_param_value_chunk(decodable, is_final);

        if is_final {
            self.cofl_action_metadata.mode = CoflMode::InToolCallBody;
            let (more, r) = self.parse_cofl_actions(&s[close_consumed..]);
            out.extend(more);
            (out, close_consumed + r)
        } else {
            (out, value_consumed)
        }
    }

    /// Emit one chunk of streamed parameter-value content.
    ///
    /// `chunk` is the raw bytes between the opening and closing
    /// `<cofl:tool_param>` tags that have arrived so far. `is_final` is
    /// `true` only on the chunk that contains the closing tag, in which
    /// case a trailing `"` is appended for string-valued params (the
    /// opening `"` was emitted eagerly in [`Self::handle_cofl_open_param`]).
    ///
    /// String-valued chunks are JSON-escaped independently. This is safe
    /// because every JSON escape we emit (`\"`, `\\`, `\n`, `\uXXXX`, ...)
    /// is produced from exactly one input character, so splitting at any
    /// UTF-8 character boundary still concatenates back to valid JSON.
    ///
    /// The chunk is emitted as a `param_delta` when
    /// `stream_processed_params` is set, otherwise as a `raw_param_delta`.
    /// Only one of the two streams is populated, matching the JSON action
    /// parser's convention.
    fn emit_cofl_param_value_chunk(&mut self, chunk: &str, is_final: bool) -> Vec<FilterOutput> {
        if !self.stream_tool_actions {
            return Vec::new();
        }
        // Value bytes may already have been consumed when the close tag completes
        // on a later call; still emit the closing `"` for string params.
        if chunk.is_empty() && !is_final {
            return Vec::new();
        }

        let decoded = if self.cofl_decode_xml_text {
            decode_xml_entities(chunk)
        } else {
            chunk.to_string()
        };

        let mut delta = if self.cofl_action_metadata.cur_param_is_string {
            json_escape_string_content(&decoded)
        } else {
            decoded
        };
        if is_final && self.cofl_action_metadata.cur_param_is_string {
            delta.push('"');
        }

        if delta.is_empty() {
            return Vec::new();
        }

        let tool_call_delta = if self.stream_processed_params {
            FilterToolCallDelta {
                index: self.cofl_action_metadata.cur_tool_call_index,
                param_delta: Some(FilterToolParameter {
                    name: self.cofl_action_metadata.cur_param_name.clone(),
                    value_delta: delta,
                }),
                ..Default::default()
            }
        } else {
            FilterToolCallDelta {
                index: self.cofl_action_metadata.cur_tool_call_index,
                raw_param_delta: delta,
                ..Default::default()
            }
        };

        vec![FilterOutput {
            tool_call_delta: Some(tool_call_delta),
            ..Default::default()
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::filter::FilterImpl;

    fn fresh_filter() -> FilterImpl {
        let mut filter = FilterImpl::new();
        filter.stream_tool_actions = true;
        filter.cofl_tool_action = true;
        filter
    }

    fn fresh_processed_filter() -> FilterImpl {
        let mut filter = fresh_filter();
        filter.stream_processed_params = true;
        filter
    }

    fn fresh_filter_no_xml_text_decode() -> FilterImpl {
        let mut filter = fresh_filter();
        filter.cofl_decode_xml_text = false;
        filter
    }

    #[test]
    fn test_parse_cofl_actions_no_xml_text_decode_unescaped_body() {
        let mut f = fresh_filter_no_xml_text_decode();
        let input = r#"<cofl:tool_call id="0" name="run&lt;cmd&gt;&amp;tool"><cofl:tool_param name="str_param" string="true">value with <tag> & "quotes"</cofl:tool_param><cofl:tool_param name="list_param" string="false">["a<b", "c&d"]</cofl:tool_param><cofl:tool_param name="param&lt;&gt;&amp;name" string="true">attr test</cofl:tool_param></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);

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
        assert_eq!(name, Some("run<cmd>&tool"));

        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        assert_eq!(parsed["str_param"], "value with <tag> & \"quotes\"");
        assert_eq!(parsed["list_param"], serde_json::json!(["a<b", "c&d"]));
        assert_eq!(parsed["param<>&name"], "attr test");
    }

    #[test]
    fn test_parse_cofl_actions_no_xml_text_decode_preserves_entities_in_body() {
        let mut f = fresh_filter_no_xml_text_decode();
        let input = r#"<cofl:tool_call id="0" name="search"><cofl:tool_param name="snippet" string="true">literal &lt;entity&gt;</cofl:tool_param></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);
        assert_eq!(consumed, input.len());

        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        assert_eq!(parsed["snippet"], "literal &lt;entity&gt;");
    }

    #[test]
    fn test_extract_attr_basic() {
        assert_eq!(extract_attr(r#"id="0" name="search""#, "id"), Some("0"));
        assert_eq!(
            extract_attr(r#"id="0" name="search""#, "name"),
            Some("search")
        );
        assert_eq!(extract_attr(r#"id="0""#, "missing"), None);
    }

    #[test]
    fn test_extract_attr_xml_encoded() {
        assert_eq!(
            extract_attr(r#"name="run&lt;cmd&gt;&amp;tool""#, "name"),
            Some("run&lt;cmd&gt;&amp;tool")
        );
        assert_eq!(
            extract_attr(r#"name="param&lt;&gt;&amp;name""#, "name"),
            Some("param&lt;&gt;&amp;name")
        );
    }

    #[test]
    fn test_extract_attr_unterminated() {
        // No closing quote -> None (incomplete attribute).
        assert_eq!(extract_attr(r#"id="0"#, "id"), None);
    }

    #[test]
    fn test_decode_xml_entities() {
        assert_eq!(decode_xml_entities("hello"), "hello");
        assert_eq!(decode_xml_entities("a&lt;b"), "a<b");
        assert_eq!(decode_xml_entities("a&amp;b"), "a&b");
        assert_eq!(decode_xml_entities("a&gt;b"), "a>b");
        assert_eq!(decode_xml_entities("&quot;hi&quot;"), "\"hi\"");
        assert_eq!(
            decode_xml_entities("value with &lt;tag&gt; &amp; \"quotes\""),
            "value with <tag> & \"quotes\""
        );
        assert_eq!(
            decode_xml_entities(r#"["a&lt;b", "c&amp;d"]"#),
            r#"["a<b", "c&d"]"#
        );
    }

    #[test]
    fn test_split_xml_entity_holdback() {
        assert_eq!(split_xml_entity_holdback("abc", false), ("abc", ""));
        assert_eq!(
            split_xml_entity_holdback("abc&amp;", false),
            ("abc&amp;", "")
        );
        assert_eq!(split_xml_entity_holdback("abc&amp", false), ("abc", "&amp"));
        assert_eq!(split_xml_entity_holdback("abc&l", false), ("abc", "&l"));
        assert_eq!(split_xml_entity_holdback("abc&l", true), ("abc&l", ""));
    }

    #[test]
    fn test_json_escape_string_content() {
        assert_eq!(json_escape_string_content("hello"), "hello");
        assert_eq!(json_escape_string_content("a\"b"), "a\\\"b");
        assert_eq!(json_escape_string_content("a\\b"), "a\\\\b");
        assert_eq!(json_escape_string_content("a\nb"), "a\\nb");
        assert_eq!(json_escape_string_content("a\x01b"), "a\\u0001b");
    }

    #[test]
    fn test_parse_cofl_actions_empty_tool_call() {
        let mut f = fresh_filter();
        let input = r#"<cofl:tool_call id="0" name="GetReminders"></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);

        assert_eq!(consumed, input.len());
        // Expect: id chunk, name chunk, raw_param "{}" chunk.
        let id_deltas: Vec<&str> = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.id.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        let name_deltas: Vec<&str> = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.name.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();

        assert_eq!(id_deltas, vec!["0"]);
        assert_eq!(name_deltas, vec!["GetReminders"]);
        assert_eq!(raw, "{}");
        assert_eq!(f.cofl_action_metadata.cur_tool_call_index, 1);
    }

    #[test]
    fn test_parse_cofl_actions_single_string_param_raw_mode() {
        let mut f = fresh_filter();
        let input = r#"<cofl:tool_call id="0" name="search"><cofl:tool_param name="q" string="true">hello</cofl:tool_param></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);

        assert_eq!(consumed, input.len());

        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        assert_eq!(raw, r#"{"q": "hello"}"#);

        // In raw mode no param_delta should be emitted at all.
        let any_param_delta = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .any(|d| d.param_delta.is_some());
        assert!(!any_param_delta);
    }

    #[test]
    fn test_parse_cofl_actions_single_string_param_processed_mode() {
        let mut f = fresh_processed_filter();
        let input = r#"<cofl:tool_call id="0" name="search"><cofl:tool_param name="q" string="true">hello</cofl:tool_param></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);

        assert_eq!(consumed, input.len());

        // In processed mode no raw_param_delta should be emitted.
        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        assert!(raw.is_empty(), "unexpected raw_param_delta: {raw:?}");

        // Aggregated value_delta should be the JSON-encoded string.
        let value: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .filter_map(|d| d.param_delta.as_ref())
            .map(|p| p.value_delta.as_str())
            .collect();
        assert_eq!(value, "\"hello\"");
    }

    #[test]
    fn test_parse_cofl_actions_single_non_string_param() {
        let mut f = fresh_filter();
        let input = r#"<cofl:tool_call id="0" name="set"><cofl:tool_param name="limit" string="false">3</cofl:tool_param></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);

        assert_eq!(consumed, input.len());

        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        assert_eq!(raw, r#"{"limit": 3}"#);
    }

    #[test]
    fn test_parse_cofl_actions_multiple_params_mixed_types() {
        let mut f = fresh_filter();
        let input = r#"<cofl:tool_call id="0" name="DeleteReminder"><cofl:tool_param name="reminder_id" string="true">12-abc</cofl:tool_param><cofl:tool_param name="force" string="false">true</cofl:tool_param><cofl:tool_param name="limit" string="false">3</cofl:tool_param></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);

        assert_eq!(consumed, input.len());

        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        assert_eq!(
            raw,
            r#"{"reminder_id": "12-abc", "force": true, "limit": 3}"#
        );
    }

    /// Exercises every branch of [`json_escape_string_content`]: the named
    /// short escapes (`\"`, `\\`, `\n`, `\r`, `\t`, `\b`, `\f`) plus a
    /// generic `\uXXXX` escape for a control char without a dedicated form
    /// (`U+0001`). Quotes and backslashes are not XML-escaped in text bodies
    /// per the cmd5 template's `xml_text` macro.
    #[test]
    fn test_parse_cofl_actions_string_value_escapes_special_chars() {
        let mut f = fresh_filter();
        let raw_value = "she said \"hi\" \\o/\n\r\t\x08\x0c\x01";
        let input = format!(
            r#"<cofl:tool_call id="0" name="echo"><cofl:tool_param name="q" string="true">{raw_value}</cofl:tool_param></cofl:tool_call>"#
        );
        let (out, consumed) = f.parse_cofl_actions(&input);

        assert_eq!(consumed, input.len());

        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        // The synthesized JSON should use every escape variant we support.
        assert_eq!(raw, r#"{"q": "she said \"hi\" \\o/\n\r\t\b\f\u0001"}"#);
        // And the synthesized JSON should decode back to the original bytes.
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        assert_eq!(
            parsed["q"],
            serde_json::Value::String(raw_value.to_string())
        );
    }

    #[test]
    fn test_parse_cofl_actions_partial_open_tag_returns_zero() {
        let mut f = fresh_filter();
        // Buffer ends mid-attribute; we should not consume anything.
        let input = r#"<cofl:tool_call id="0" name="sea"#;
        let (out, consumed) = f.parse_cofl_actions(input);

        assert_eq!(consumed, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn test_parse_cofl_actions_partial_value_streams_chunk() {
        let mut f = fresh_filter();
        let prefix =
            r#"<cofl:tool_call id="0" name="search"><cofl:tool_param name="q" string="true">"#;
        let _ = f.parse_cofl_actions(prefix);

        // A partial value (no close tag yet) should stream what we have.
        let (out, consumed) = f.parse_cofl_actions("hello");
        assert_eq!(consumed, 5);
        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        assert_eq!(raw, "hello");

        // The remainder of the value plus the close tag completes the param.
        let (out, consumed) = f.parse_cofl_actions(" world</cofl:tool_param></cofl:tool_call>");
        assert!(consumed > 0);
        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        assert_eq!(raw, r#" world"}"#);
    }

    #[test]
    fn test_parse_cofl_actions_xml_entity_escaping() {
        let mut f = fresh_filter();
        let input = r#"<cofl:tool_call id="0" name="run&lt;cmd&gt;&amp;tool"><cofl:tool_param name="str_param" string="true">value with &lt;tag&gt; &amp; "quotes"</cofl:tool_param><cofl:tool_param name="num_param" string="false">42</cofl:tool_param><cofl:tool_param name="list_param" string="false">["a&lt;b", "c&amp;d"]</cofl:tool_param><cofl:tool_param name="param&lt;&gt;&amp;name" string="true">attr test</cofl:tool_param><cofl:tool_param name="nested" string="false">{"key&lt;1&gt;": "val&gt;2"}</cofl:tool_param></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);

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
        assert_eq!(name, Some("run<cmd>&tool"));

        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        assert_eq!(parsed["str_param"], "value with <tag> & \"quotes\"");
        assert_eq!(parsed["num_param"], 42);
        assert_eq!(parsed["list_param"], serde_json::json!(["a<b", "c&d"]));
        assert_eq!(parsed["param<>&name"], "attr test");
        assert_eq!(parsed["nested"], serde_json::json!({"key<1>": "val>2"}));
    }

    #[test]
    fn test_parse_cofl_actions_streaming_xml_entity_split() {
        let mut f = fresh_filter();
        let prefix =
            r#"<cofl:tool_call id="0" name="run"><cofl:tool_param name="snippet" string="true">"#;
        let _ = f.parse_cofl_actions(prefix);

        // Simulate the filter buffer: `&lt;` split across chunks leaves `&l`
        // unconsumed, same as a partial close tag.
        let mut buf = "if (a &l".to_string();
        let (out, consumed) = f.parse_cofl_actions(&buf);
        assert_eq!(consumed, 6);
        buf.drain(..consumed);
        assert_eq!(buf, "&l");
        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        assert_eq!(raw, "if (a ");

        buf.push_str("t; b)</cofl:tool_param></cofl:tool_call>");
        let (out, consumed) = f.parse_cofl_actions(&buf);
        assert_eq!(consumed, buf.len());
        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        assert!(raw.starts_with(r#"< b)""#), "unexpected raw: {raw:?}");

        let mut f2 = fresh_filter();
        let input = r#"<cofl:tool_call id="0" name="run"><cofl:tool_param name="snippet" string="true">if (a &lt; b)</cofl:tool_param></cofl:tool_call>"#;
        let (out, _) = f2.parse_cofl_actions(input);
        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        assert_eq!(parsed["snippet"], "if (a < b)");
    }

    #[test]
    fn test_parse_cofl_actions_partial_close_tag_held_in_buffer() {
        let mut f = fresh_filter();
        let prefix =
            r#"<cofl:tool_call id="0" name="search"><cofl:tool_param name="q" string="true">"#;
        let _ = f.parse_cofl_actions(prefix);

        // Buffer ends mid-close-tag. The streamed value must stop before
        // the partial tag and the partial bytes must remain unconsumed so
        // the next call can complete the match.
        let input = "abc</cofl:tool_p";
        let (out, consumed) = f.parse_cofl_actions(input);
        assert_eq!(consumed, 3);
        let raw: String = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.raw_param_delta.as_str())
            .collect();
        assert_eq!(raw, "abc");
    }

    #[test]
    fn test_parse_cofl_actions_two_tool_calls_body() {
        // Two adjacent tool calls in a single buffer. The first close-tag
        // appears before any param-open in the second call, so the body
        // dispatcher must pick the close first.
        let mut f = fresh_filter();
        let input = r#"<cofl:tool_call id="0" name="GetReminders"></cofl:tool_call><cofl:tool_call id="1" name="GetTodos"><cofl:tool_param name="filter" string="true">open</cofl:tool_param></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);

        assert_eq!(consumed, input.len());
        assert_eq!(f.cofl_action_metadata.cur_tool_call_index, 2);

        // Collect indices observed for tool_call deltas.
        let indices: Vec<usize> = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.index)
            .collect();
        assert!(indices.contains(&0));
        assert!(indices.contains(&1));
    }

    #[test]
    fn test_parse_cofl_actions_multiple_tool_calls_advance_index() {
        let mut f = fresh_filter();
        let input = r#"<cofl:tool_call id="0" name="a"></cofl:tool_call><cofl:tool_call id="1" name="b"></cofl:tool_call>"#;
        let (out, consumed) = f.parse_cofl_actions(input);

        assert_eq!(consumed, input.len());
        assert_eq!(f.cofl_action_metadata.cur_tool_call_index, 2);

        // Collect indices observed for tool_call deltas.
        let indices: Vec<usize> = out
            .iter()
            .filter_map(|o| o.tool_call_delta.as_ref())
            .map(|d| d.index)
            .collect();
        assert!(indices.contains(&0));
        assert!(indices.contains(&1));
    }

    #[test]
    fn test_parse_cofl_actions_streaming_in_pieces() {
        // Splitting the same input at arbitrary character boundaries should
        // produce the same aggregate raw_param_delta.
        let full = r#"<cofl:tool_call id="0" name="search"><cofl:tool_param name="q" string="true">hello</cofl:tool_param></cofl:tool_call>"#;

        let mut combined_raw = String::new();
        let mut f = fresh_filter();
        let mut buf = String::new();
        for c in full.chars() {
            buf.push(c);
            let (out, consumed) = f.parse_cofl_actions(&buf);
            for o in out {
                if let Some(d) = o.tool_call_delta {
                    combined_raw.push_str(&d.raw_param_delta);
                }
            }
            buf.drain(..consumed);
        }
        // Any leftover should be empty since the input is complete.
        assert!(buf.is_empty(), "leftover buffer: {buf:?}");
        assert_eq!(combined_raw, r#"{"q": "hello"}"#);
    }
}
