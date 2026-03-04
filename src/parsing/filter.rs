//! Core filtering logic and state machine implementation
//!
//! This module contains the main filter implementation that processes streaming tokens
//! and extracts structured information, as well as aggregation functions for efficient interop.

use crate::parsing::action_filter::FilterAction;
use crate::parsing::options::FilterOptions;
use crate::parsing::types::{
    AccumulatedToolCall, FilterAggregatedResult, FilterMode, FilterOutput, FilterSearchQueryDelta,
    SearchQueryDelta,
};
use std::collections::HashMap;

fn push_text(target: &mut Option<String>, text: &mut String) {
    match target {
        Some(s) => s.push_str(text),
        None => {
            *target = Some(std::mem::take(text));
        }
    }
}

#[must_use]
pub(crate) fn aggregate(outputs: Vec<FilterOutput>) -> FilterAggregatedResult {
    let mut content: Option<String> = None;
    let mut reasoning: Option<String> = None;
    let mut tool_call_map: HashMap<usize, AccumulatedToolCall> = HashMap::new();
    let mut citations = Vec::new();
    let mut search_queries = Vec::new();

    for mut o in outputs {
        if !o.text.is_empty() {
            let target = if o.is_reasoning {
                &mut reasoning
            } else {
                &mut content
            };
            push_text(target, &mut o.text);
        }
        if !o.citations.is_empty() {
            citations.append(&mut o.citations);
        }
        if let Some(tc) = o.tool_call_delta {
            let call = tool_call_map
                .entry(tc.index)
                .or_insert_with(|| AccumulatedToolCall {
                    index: tc.index,
                    ..Default::default()
                });
            if !tc.id.is_empty() {
                call.id = tc.id;
            }
            if !tc.name.is_empty() {
                call.name = tc.name;
            }
            call.arguments.push_str(&tc.raw_param_delta);
            if let Some(param_delta) = tc.param_delta {
                if let Some(last_param) = call.processed_params.last_mut() {
                    if last_param.name == param_delta.name {
                        last_param.value_delta.push_str(&param_delta.value_delta);
                    } else {
                        call.processed_params.push(param_delta);
                    }
                } else {
                    call.processed_params.push(param_delta);
                }
            }
        }
        if let Some(sq) = o.search_query {
            search_queries.push(SearchQueryDelta {
                index: sq.index,
                text: sq.text,
            });
        }
    }

    let mut tool_calls: Vec<AccumulatedToolCall> = tool_call_map.into_values().collect();
    tool_calls.sort_by_key(|tc| tc.index);

    FilterAggregatedResult {
        content,
        reasoning,
        tool_calls,
        citations,
        search_queries,
    }
}

// ---------------------------------------------------------------------------
// Filter trait and implementation
// ---------------------------------------------------------------------------

/// Trait for streaming token filters that return aggregated results.
pub trait Filter {
    /// Process a decoded token and return an aggregated result.
    fn write_decoded(&mut self, decoded_token: &str) -> FilterAggregatedResult;

    /// Flush any buffered partial outputs.
    fn flush_partials(&mut self) -> FilterAggregatedResult;
}

/// Main implementation of the streaming filter state machine.
///
/// This struct maintains all the state needed to incrementally parse token streams,
/// including:
/// - Current parsing mode and mode transitions
/// - Buffered content waiting for complete structures
/// - Position tracking for citations
/// - Configuration options
///
/// # Implementation Notes
///
/// The filter operates as a state machine that:
/// 1. Buffers incoming tokens until they form complete UTF-8 sequences
/// 2. Checks for special tokens that trigger mode transitions
/// 3. Processes content based on the current mode
/// 4. Outputs structured results when complete chunks are available
///
/// # Internal State
///
/// This struct contains many fields to track various aspects of parsing. Users should
/// not create instances directly; use `new_filter()` instead.
#[allow(clippy::struct_excessive_bools)]
pub struct FilterImpl {
    // Trimming configuration
    pub(crate) left_trimmed: bool,
    pub(crate) right_trimmed: bool,

    // Mode and special token configuration
    pub(crate) default_mode: FilterMode,
    pub(crate) special_token_map: HashMap<String, FilterMode>,
    pub(crate) stream_non_grounded_answer: bool,
    pub(crate) stream_tool_actions: bool,
    pub(crate) stream_processed_params: bool,

    // Raw parameter parsing state
    pub(crate) raw_param_indent_length_removed: usize,
    pub(crate) saw_non_whitespace_in_current_line: bool,

    // Citation tracking
    pub(crate) cur_text_index: usize,
    pub(crate) cur_text_byte_index: usize,
    pub(crate) cur_citation_byte_index: Option<usize>,
    pub(crate) action_metadata: FilterAction,

    // Search query tracking
    pub(crate) curr_search_query_idx: usize,
    pub(crate) sent_curr_index: bool,

    // Format flags
    pub(crate) has_tool_call_id: bool,
    pub(crate) cmd3_citations: bool,

    // Chunking configuration
    pub(crate) chunk_size: usize,
    pub(crate) num_tokens_in_chunk: usize,

    // Buffering state
    pub(crate) buf: Vec<u8>,
    pub(crate) mode: FilterMode,
    pub(crate) done: bool,
}

impl FilterImpl {
    pub(crate) fn new() -> Self {
        Self {
            left_trimmed: false,
            right_trimmed: false,
            default_mode: FilterMode::PlainText,
            special_token_map: HashMap::new(),
            stream_non_grounded_answer: false,
            stream_tool_actions: false,
            stream_processed_params: false,
            raw_param_indent_length_removed: 0,
            saw_non_whitespace_in_current_line: false,
            cur_text_index: 0,
            cur_text_byte_index: 0,
            cur_citation_byte_index: None,
            action_metadata: FilterAction::new(),
            curr_search_query_idx: 0,
            sent_curr_index: false,
            has_tool_call_id: false,
            cmd3_citations: false,
            chunk_size: 1,
            num_tokens_in_chunk: 0,
            buf: Vec::new(),
            mode: FilterMode::PlainText,
            done: false,
        }
    }

    pub(crate) fn apply_options(mut self, options: FilterOptions) -> Self {
        self.left_trimmed = options.left_trimmed;
        self.right_trimmed = options.right_trimmed;
        self.chunk_size = options.chunk_size;
        self.stream_non_grounded_answer = options.stream_non_grounded_answer;
        self.stream_tool_actions = options.stream_tool_actions;
        self.stream_processed_params = options.stream_processed_params;
        self.has_tool_call_id = options.has_tool_call_id;
        self.cmd3_citations = options.cmd3_citations;
        self.default_mode = options.default_mode;
        self.mode = options.default_mode;

        // Merge special token maps
        for (token, mode) in &options.special_token_map {
            self.special_token_map.insert(token.clone(), *mode);
        }

        // Add inclusive stops
        for stop in options.inclusive_stops {
            self.special_token_map
                .insert(stop, FilterMode::InclusiveStop);
        }

        // Add exclusive stops
        for stop in options.exclusive_stops {
            self.special_token_map
                .insert(stop, FilterMode::ExclusiveStop);
        }

        self
    }

    pub(crate) fn write_text(&mut self, text: &[u8]) -> Vec<FilterOutput> {
        if self.done {
            return Vec::new();
        }

        self.buf.extend_from_slice(text);
        let str = String::from_utf8_lossy(&self.buf).to_string();

        // If is a partial special token, we need to wait for the next token.
        let (special_token_idx, found_seq) = find_partial(&str, &mut self.special_token_map.keys());
        if special_token_idx != usize::MAX && found_seq.is_empty() {
            return Vec::new();
        }

        let mut out = Vec::new();

        // If it is a whole special token, change the mode, remove the tokens and continue
        if special_token_idx != usize::MAX && !found_seq.is_empty() {
            let (o, new_mode, stop, valid_special) =
                self.handle_special_token(&str, special_token_idx, &found_seq, self.mode);
            out.extend(o);

            if valid_special {
                if stop {
                    self.buf.clear();
                    self.done = true;
                    return out;
                }

                // Before the special token, process the buffer with the old mode
                let pre_special_token = &str[..special_token_idx];
                if !pre_special_token.is_empty() {
                    let (o, _) = self.handle_token(self.mode, pre_special_token.as_bytes(), false);
                    out.extend(o);
                }

                // Remove the special token and the text before
                let remove_len = pre_special_token.len() + found_seq.len();
                self.buf.drain(..remove_len);

                // Change mode
                self.mode = new_mode;
            }
        }

        // Process buffer by mode
        if !self.buf.is_empty() {
            self.num_tokens_in_chunk += 1;

            if self.chunk_size > 1 && self.num_tokens_in_chunk < self.chunk_size {
                return out;
            }

            let buf = std::mem::take(&mut self.buf);
            let (o, remove) = self.handle_token(self.mode, &buf, false);
            out.extend(o);
            self.buf = buf[remove..].to_vec();
            self.num_tokens_in_chunk = 0;
        }

        out
    }

    fn handle_token(
        &mut self,
        mode: FilterMode,
        bstr: &[u8],
        after_last_token: bool,
    ) -> (Vec<FilterOutput>, usize) {
        match mode {
            FilterMode::InclusiveStop | FilterMode::ExclusiveStop => {
                log::error!("in stop mode but we should have already stopped");
                (Vec::new(), 0)
            }
            FilterMode::Ignore | FilterMode::NextSearchQuery => (Vec::new(), 0),
            FilterMode::ToolAction => {
                let s = String::from_utf8_lossy(bstr);
                self.parse_actions(&s)
            }
            FilterMode::GroundedAnswer | FilterMode::ToolReason => {
                self.process_grounded_text(bstr, after_last_token, mode)
            }
            FilterMode::SearchQuery => self.process_search_query(bstr),
            FilterMode::Answer => {
                if self.stream_non_grounded_answer {
                    self.process_text(bstr)
                } else {
                    (Vec::new(), bstr.len())
                }
            }
            FilterMode::PlainText => self.process_text(bstr),
        }
    }

    fn handle_special_token(
        &mut self,
        s: &str,
        idx: usize,
        token: &str,
        cur_mode: FilterMode,
    ) -> (Vec<FilterOutput>, FilterMode, bool, bool) {
        let new_mode = self
            .special_token_map
            .get(token)
            .copied()
            .unwrap_or(FilterMode::PlainText);

        // Disable mode change if in grounded answer or answer mode and see "Answer:" in the text
        let not_special = (cur_mode == FilterMode::GroundedAnswer
            || cur_mode == FilterMode::Answer)
            && new_mode == FilterMode::Answer;

        if not_special {
            return (Vec::new(), cur_mode, false, false);
        }

        match new_mode {
            FilterMode::InclusiveStop => {
                let out = self.handle_inclusive_stop(s, idx, token);
                (out, new_mode, true, true)
            }
            FilterMode::ExclusiveStop => {
                let out = self.handle_exclusive_stop(s, idx);
                (out, new_mode, true, true)
            }
            FilterMode::GroundedAnswer => {
                self.cur_text_index = 0;
                if self.stream_non_grounded_answer {
                    self.left_trimmed = true;
                }
                (Vec::new(), new_mode, false, true)
            }
            FilterMode::ToolReason => {
                self.left_trimmed = true;
                self.right_trimmed = true;
                (Vec::new(), new_mode, false, true)
            }
            FilterMode::Answer | FilterMode::SearchQuery => {
                self.left_trimmed = true;
                (Vec::new(), new_mode, false, true)
            }
            FilterMode::NextSearchQuery => {
                self.left_trimmed = true;
                if self.sent_curr_index {
                    self.curr_search_query_idx += 1;
                    self.sent_curr_index = false;
                }
                (Vec::new(), FilterMode::SearchQuery, false, true)
            }
            _ => (Vec::new(), new_mode, false, true),
        }
    }

    pub(crate) fn handle_inclusive_stop(
        &self,
        s: &str,
        idx: usize,
        token: &str,
    ) -> Vec<FilterOutput> {
        if idx != usize::MAX && !s[..idx + token.len()].is_empty() {
            let text = if let Some(start_idx) = self.cur_citation_byte_index {
                s[start_idx..idx + token.len()].to_string()
            } else {
                s[..idx + token.len()].to_string()
            };

            return vec![FilterOutput {
                text,
                ..Default::default()
            }];
        }
        Vec::new()
    }

    pub(crate) fn handle_exclusive_stop(&mut self, s: &str, idx: usize) -> Vec<FilterOutput> {
        if idx != usize::MAX && !s[..idx].is_empty() {
            let text = if let Some(start_idx) = self.cur_citation_byte_index {
                let (trimmed, _) = self.trim_space(&s[start_idx..idx]);
                trimmed.to_string()
            } else {
                let (trimmed, _) = self.trim_space(&s[..idx]);
                trimmed.to_string()
            };

            return vec![FilterOutput {
                text,
                ..Default::default()
            }];
        }
        Vec::new()
    }

    pub(crate) fn utf8_valid_or_limit(bstr: &[u8]) -> bool {
        let limit = 4; // utf-8 is up to 4 bytes
        let valid = std::str::from_utf8(bstr).is_ok();
        if bstr.len() >= limit && !valid {
            log::warn!("emitting invalid utf8: {bstr:?}");
        }
        valid || bstr.len() >= limit
    }

    pub(crate) fn process_search_query(&mut self, bstr: &[u8]) -> (Vec<FilterOutput>, usize) {
        if !Self::utf8_valid_or_limit(bstr) {
            return (Vec::new(), 0);
        }

        let s = String::from_utf8_lossy(bstr);
        let (send, rem_right) = self.trim_space(&s);
        let mut out = Vec::new();

        if !send.is_empty() {
            out.push(FilterOutput {
                search_query: Some(FilterSearchQueryDelta {
                    index: self.curr_search_query_idx,
                    text: send.to_string(),
                }),
                ..Default::default()
            });
            self.sent_curr_index = true;
        }

        (out, bstr.len() - rem_right)
    }

    pub(crate) fn process_text(&mut self, bstr: &[u8]) -> (Vec<FilterOutput>, usize) {
        if !Self::utf8_valid_or_limit(bstr) {
            return (Vec::new(), 0);
        }

        let s = String::from_utf8_lossy(bstr);
        let (send, rem_right) = self.trim_space(&s);
        let mut out = Vec::new();

        if !send.is_empty() {
            out.push(FilterOutput {
                text: send.to_string(),
                ..Default::default()
            });
        }

        (out, bstr.len() - rem_right)
    }

    pub(crate) fn trim_space<'a>(&mut self, s: &'a str) -> (&'a str, usize) {
        let mut result = s;
        let mut rem = 0;

        if self.right_trimmed {
            let trimmed = result.trim_end();
            rem = result.len() - trimmed.len();
            result = trimmed;
        }

        if self.left_trimmed {
            result = result.trim_start();
            if !result.is_empty() {
                self.left_trimmed = false;
            }
        }

        (result, rem)
    }
}

impl FilterImpl {
    /// Feed all tokens through the filter and return a single result with fully accumulated tool calls.
    pub fn process_full(&mut self, token_strings: &[String]) -> FilterAggregatedResult {
        let mut all_outputs = Vec::with_capacity(token_strings.len());
        for token_str in token_strings {
            all_outputs.extend(self.write_text(token_str.as_bytes()));
        }
        self.done = true;
        if !self.buf.is_empty()
            && self.mode != FilterMode::InclusiveStop
            && self.mode != FilterMode::ExclusiveStop
        {
            let buf_copy = std::mem::take(&mut self.buf);
            let (o, _) = self.handle_token(self.mode, &buf_copy, true);
            all_outputs.extend(o);
        }
        aggregate(all_outputs)
    }
}

impl Filter for FilterImpl {
    fn write_decoded(&mut self, decoded_token: &str) -> FilterAggregatedResult {
        let outputs = self.write_text(decoded_token.as_bytes());
        aggregate(outputs)
    }

    fn flush_partials(&mut self) -> FilterAggregatedResult {
        self.done = true;
        if !self.buf.is_empty()
            && self.mode != FilterMode::InclusiveStop
            && self.mode != FilterMode::ExclusiveStop
        {
            let buf_copy = std::mem::take(&mut self.buf);
            let (o, _remove) = self.handle_token(self.mode, &buf_copy, true);
            return aggregate(o);
        }
        FilterAggregatedResult::default()
    }
}

/// Find partial returns first index in str that might match one of stop sequences.
pub(crate) fn find_partial<'a>(
    s: &str,
    stops: impl Iterator<Item = &'a String>,
) -> (usize, String) {
    let mut min_idx = usize::MAX;

    for stop in stops {
        // If we find the stop sequence, return the index and the stop sequence
        if let Some(idx) = s.find(stop) {
            return (idx, stop.clone());
        }
        // Go through the substrings of the stop sequence
        'inner: for i in 0..stop.len() {
            if !stop.is_char_boundary(stop.len() - i) {
                continue 'inner;
            }
            let suffix = &stop[..stop.len() - i];

            if s.ends_with(suffix) {
                let idx = s.len() - suffix.len();
                if min_idx == usize::MAX || min_idx > idx {
                    min_idx = idx;
                }
                break;
            }
        }
    }

    (
        if min_idx == usize::MAX {
            usize::MAX
        } else {
            min_idx
        },
        String::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::types::{FilterCitation, FilterToolCallDelta, FilterToolParameter};

    #[test]
    fn test_find_partial() {
        let stops = vec!["<co: ".to_string(), "</co: ".to_string()];

        // Test full match
        let (idx, found) = find_partial("hello <co: ", stops.iter());
        assert_eq!(idx, 6);
        assert_eq!(found, "<co: ");

        // Test partial match
        let (idx, found) = find_partial("hello <c", stops.iter());
        assert_eq!(idx, 6);
        assert_eq!(found, "");

        // Test no match
        let (idx, _) = find_partial("hello world", stops.iter());
        assert_eq!(idx, usize::MAX);
    }

    #[test]
    fn test_find_partial_utf8() {
        // This test ensures we don't slice in the middle of a UTF-8 character (we used to panic here).
        let stops = vec!["RÈGLES".to_string()];
        let (idx, found) = find_partial("ÈÈÈÈÈÈÈR", stops.iter());
        assert_eq!(idx, 14);
        assert_eq!(found, "");
    }

    #[test]
    fn test_aggregate_content_only() {
        let outputs = vec![
            FilterOutput {
                text: "hello ".into(),
                ..Default::default()
            },
            FilterOutput {
                text: "world".into(),
                ..Default::default()
            },
        ];
        let result = aggregate(outputs);
        assert_eq!(result.content, Some("hello world".into()));
        assert!(result.reasoning.is_none());
        assert!(result.tool_calls.is_empty());
    }

    #[test]
    fn test_aggregate_reasoning_only() {
        let outputs = vec![
            FilterOutput {
                text: "step 1".into(),
                is_reasoning: true,
                ..Default::default()
            },
            FilterOutput {
                text: " step 2".into(),
                is_reasoning: true,
                ..Default::default()
            },
        ];
        let result = aggregate(outputs);
        assert!(result.content.is_none());
        assert_eq!(result.reasoning, Some("step 1 step 2".into()));
    }

    #[test]
    fn test_aggregate_mixed() {
        let outputs = vec![
            FilterOutput {
                text: "thinking...".into(),
                is_reasoning: true,
                ..Default::default()
            },
            FilterOutput {
                text: "hello".into(),
                is_reasoning: false,
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    id: "call_0".into(),
                    name: "search".into(),
                    raw_param_delta: r#"{"q":"test"}"#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];

        let result = aggregate(outputs);
        assert_eq!(result.reasoning, Some("thinking...".into()));
        assert_eq!(result.content, Some("hello".into()));
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_0");
        assert_eq!(result.tool_calls[0].arguments, r#"{"q":"test"}"#);
    }

    #[test]
    fn test_aggregate_with_citations() {
        let outputs = vec![FilterOutput {
            text: "cited text".into(),
            citations: vec![FilterCitation {
                start_index: 0,
                end_index: 10,
                text: "cited text".into(),
                sources: vec![],
                is_thinking: false,
            }],
            ..Default::default()
        }];
        let result = aggregate(outputs);
        assert_eq!(result.citations.len(), 1);
        assert_eq!(result.citations[0].start_index, 0);
    }

    #[test]
    fn test_aggregate_no_text_fields() {
        let outputs = vec![FilterOutput {
            text: String::new(),
            ..Default::default()
        }];
        let result = aggregate(outputs);
        assert!(result.content.is_none());
        assert!(result.reasoning.is_none());
    }

    #[test]
    fn test_aggregate_empty() {
        let result = aggregate(vec![]);
        assert!(result.content.is_none());
        assert!(result.reasoning.is_none());
        assert!(result.tool_calls.is_empty());
        assert!(result.citations.is_empty());
    }

    #[test]
    fn test_aggregate_tool_calls() {
        let outputs = vec![
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    id: "0".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    name: "search".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    raw_param_delta: r#"{"q": "#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    raw_param_delta: r#""hello"}"#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                text: "Response text".into(),
                ..Default::default()
            },
        ];

        let result = aggregate(outputs);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "0");
        assert_eq!(result.tool_calls[0].name, "search");
        assert_eq!(result.tool_calls[0].arguments, r#"{"q": "hello"}"#);
        assert_eq!(result.content, Some("Response text".into()));
    }

    #[test]
    fn test_aggregate_processed_params_tool_calls() {
        let outputs = vec![
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    id: "0".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    name: "search".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    param_delta: Some(FilterToolParameter {
                        name: "q".into(),
                        value_delta: "".into(),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    param_delta: Some(FilterToolParameter {
                        name: "q".into(),
                        value_delta: "\"hel".into(),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    param_delta: Some(FilterToolParameter {
                        name: "q".into(),
                        value_delta: "lo\"".into(),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                text: "Response text".into(),
                ..Default::default()
            },
        ];

        let result = aggregate(outputs);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "0");
        assert_eq!(result.tool_calls[0].name, "search");
        assert_eq!(result.tool_calls[0].arguments, r#""#);
        assert_eq!(
            result.tool_calls[0].processed_params[0],
            FilterToolParameter {
                name: "q".into(),
                value_delta: "\"hello\"".into(),
            }
        );
        assert_eq!(result.content, Some("Response text".into()));
    }

    #[test]
    fn test_aggregate_multiple_tool_calls() {
        let outputs = vec![
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    id: "call_0".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    name: "search".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 1,
                    id: "call_1".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 1,
                    name: "read".into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 0,
                    raw_param_delta: r#"{"q":"a"}"#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            FilterOutput {
                tool_call_delta: Some(FilterToolCallDelta {
                    index: 1,
                    raw_param_delta: r#"{"file":"b"}"#.into(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];

        let result = aggregate(outputs);
        assert_eq!(result.tool_calls.len(), 2);
        assert_eq!(result.tool_calls[0].id, "call_0");
        assert_eq!(result.tool_calls[0].name, "search");
        assert_eq!(result.tool_calls[0].arguments, r#"{"q":"a"}"#);
        assert_eq!(result.tool_calls[1].id, "call_1");
        assert_eq!(result.tool_calls[1].name, "read");
        assert_eq!(result.tool_calls[1].arguments, r#"{"file":"b"}"#);
    }

    #[test]
    fn test_aggregate_reasoning_and_content() {
        let outputs = vec![
            FilterOutput {
                text: "thinking".into(),
                is_reasoning: true,
                ..Default::default()
            },
            FilterOutput {
                text: "answer".into(),
                is_reasoning: false,
                ..Default::default()
            },
        ];
        let result = aggregate(outputs);
        assert_eq!(result.reasoning, Some("thinking".into()));
        assert_eq!(result.content, Some("answer".into()));
        assert!(result.tool_calls.is_empty());
    }
}
