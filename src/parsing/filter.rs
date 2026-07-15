//! Core filtering logic and state machine implementation
//!
//! This module contains the main filter implementation that processes streaming tokens
//! and extracts structured information, as well as aggregation functions for efficient interop.

use crate::parsing::action_filter::FilterAction;
use crate::parsing::cofl_filter::FilterCoflAction;
use crate::parsing::cofl_nested_filter;
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
    match outputs.len() {
        0 => FilterAggregatedResult::default(),
        1 => aggregate_single(outputs),
        _ => aggregate_general(outputs),
    }
}

fn aggregate_single(mut outputs: Vec<FilterOutput>) -> FilterAggregatedResult {
    let o = outputs.pop().unwrap();

    let (content, reasoning) = if o.text.is_empty() {
        (None, None)
    } else if o.is_reasoning {
        (None, Some(o.text))
    } else {
        (Some(o.text), None)
    };

    let tool_calls = if let Some(tc) = o.tool_call_delta {
        let mut call = AccumulatedToolCall {
            index: tc.index,
            id: tc.id,
            name: tc.name,
            arguments: tc.raw_param_delta,
            ..Default::default()
        };
        if let Some(pd) = tc.param_delta {
            call.processed_params.push(pd);
        }
        vec![call]
    } else {
        Vec::new()
    };

    let search_queries = if let Some(sq) = o.search_query {
        vec![SearchQueryDelta {
            index: sq.index,
            text: sq.text,
        }]
    } else {
        Vec::new()
    };

    FilterAggregatedResult {
        content,
        reasoning,
        tool_calls,
        citations: o.citations,
        search_queries,
    }
}

fn aggregate_general(outputs: Vec<FilterOutput>) -> FilterAggregatedResult {
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
    pub(crate) special_token_start_bytes: [bool; 256],
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
    pub(crate) cofl_action_metadata: FilterCoflAction,
    pub(crate) cofl_nested_action_metadata: cofl_nested_filter::FilterCoflNestedAction,

    // Search query tracking
    pub(crate) curr_search_query_idx: usize,
    pub(crate) sent_curr_index: bool,

    // Format flags
    pub(crate) has_tool_call_id: bool,
    pub(crate) cmd3_citations: bool,
    /// Use the cofl-tagged parser (cmd5) for [`FilterMode::ToolAction`]
    /// instead of the JSON action parser.
    pub(crate) cofl_tool_action: bool,
    /// Decode XML entities in cofl parameter bodies before emitting tool-call
    /// arguments. Attribute values are always decoded regardless of this flag.
    pub(crate) cofl_decode_xml_text: bool,
    /// Parse cofl tool parameters as nested `<cofl:value>` nodes (cmd5-nested-xml).
    pub(crate) cofl_nested_xml: bool,

    // Chunking configuration
    pub(crate) chunk_size: usize,
    pub(crate) num_tokens_in_chunk: usize,

    // Buffering state
    pub(crate) buf: Vec<u8>,
    pub(crate) mode: FilterMode,
    pub(crate) done: bool,
}

struct SpecialTokenMatch {
    idx: usize,
    sequence: String,
    decoded: String,
}

enum SpecialTokenScanResult {
    NoMatch,
    Partial,
    Found(SpecialTokenMatch),
}

/// Outcome of trying to apply a matched special token.
enum SpecialTokenOutcome {
    /// The token was consumed: buffer drained, mode updated. Keep scanning.
    Consumed,
    /// The match was rejected.
    Rejected,
    /// An inclusive/exclusive stop token fired; the filter is done.
    Stop,
}

#[derive(Debug)]
pub(crate) enum PartialMatchResult {
    NoMatch,
    Partial { idx: usize },
    Full { idx: usize, sequence: String },
}

impl FilterImpl {
    pub(crate) fn new() -> Self {
        Self {
            left_trimmed: false,
            right_trimmed: false,
            default_mode: FilterMode::PlainText,
            special_token_map: HashMap::new(),
            special_token_start_bytes: [false; 256],
            stream_non_grounded_answer: false,
            stream_tool_actions: false,
            stream_processed_params: false,
            raw_param_indent_length_removed: 0,
            saw_non_whitespace_in_current_line: false,
            cur_text_index: 0,
            cur_text_byte_index: 0,
            cur_citation_byte_index: None,
            action_metadata: FilterAction::new(),
            cofl_action_metadata: FilterCoflAction::new(),
            cofl_nested_action_metadata: cofl_nested_filter::FilterCoflNestedAction::new(),
            curr_search_query_idx: 0,
            sent_curr_index: false,
            has_tool_call_id: false,
            cmd3_citations: false,
            cofl_tool_action: false,
            cofl_decode_xml_text: true,
            cofl_nested_xml: false,
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
        self.cofl_tool_action = options.cofl_tool_action;
        self.cofl_decode_xml_text = options.cofl_decode_xml_text;
        self.cofl_nested_xml = options.cofl_nested_xml;
        self.default_mode = options.default_mode;
        self.mode = options.default_mode;

        // Merge special token maps
        for (token, mode) in &options.special_token_map {
            self.special_token_map.insert(token.clone(), *mode);
            if let Some(first_byte) = token.as_bytes().first() {
                self.special_token_start_bytes[usize::from(*first_byte)] = true;
            }
        }

        // Add inclusive stops
        for stop in options.inclusive_stops {
            if let Some(first_byte) = stop.as_bytes().first() {
                self.special_token_start_bytes[usize::from(*first_byte)] = true;
            }
            self.special_token_map
                .insert(stop, FilterMode::InclusiveStop);
        }

        // Add exclusive stops
        for stop in options.exclusive_stops {
            if let Some(first_byte) = stop.as_bytes().first() {
                self.special_token_start_bytes[usize::from(*first_byte)] = true;
            }
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
        let mut out = Vec::new();

        // a single text chunk may contain more than one special token!
        // this is because when using speculative decoding several
        // accepted tokens are concatenated into one delta.

        // loop until no special token is detected to avoid leaking
        // unconsumed tokens.
        loop {
            match self.detect_special_token() {
                SpecialTokenScanResult::Partial => return out,
                SpecialTokenScanResult::NoMatch => break,
                SpecialTokenScanResult::Found(token_match) => {
                    match self.apply_special_token_match(&token_match, &mut out) {
                        SpecialTokenOutcome::Consumed => {}
                        SpecialTokenOutcome::Stop => return out,
                        // Rejected matches (e.g. `Answer:` while already in
                        // GroundedAnswer) leave buf and mode untouched, so
                        // rescanning would loop forever on the same hit.
                        // Fall through to plain buffer processing.
                        SpecialTokenOutcome::Rejected => break,
                    }
                }
            }
        }

        // Process buffer by mode
        if !self.buf.is_empty() {
            self.num_tokens_in_chunk += 1;

            if self.chunk_size > 1 && self.num_tokens_in_chunk < self.chunk_size {
                return out;
            }

            let mut buf = std::mem::take(&mut self.buf);
            let (o, remove) = self.handle_token(self.mode, &buf, false);
            out.extend(o);
            if remove > 0 {
                buf.drain(..remove);
            }
            self.buf = buf;
            self.num_tokens_in_chunk = 0;
        }

        out
    }

    fn detect_special_token(&self) -> SpecialTokenScanResult {
        if !self
            .buf
            .iter()
            .any(|byte| self.special_token_start_bytes[usize::from(*byte)])
        {
            return SpecialTokenScanResult::NoMatch;
        }

        let decoded = String::from_utf8_lossy(&self.buf).into_owned();
        match find_partial(&decoded, self.special_token_map.keys()) {
            PartialMatchResult::NoMatch => SpecialTokenScanResult::NoMatch,
            PartialMatchResult::Partial { .. } => SpecialTokenScanResult::Partial,
            PartialMatchResult::Full { idx, sequence } => {
                SpecialTokenScanResult::Found(SpecialTokenMatch {
                    idx,
                    sequence,
                    decoded,
                })
            }
        }
    }

    fn apply_special_token_match(
        &mut self,
        token_match: &SpecialTokenMatch,
        out: &mut Vec<FilterOutput>,
    ) -> SpecialTokenOutcome {
        let (o, new_mode, stop, valid_special) = self.handle_special_token(
            &token_match.decoded,
            token_match.idx,
            &token_match.sequence,
            self.mode,
        );
        out.extend(o);

        if !valid_special {
            return SpecialTokenOutcome::Rejected;
        }

        if stop {
            self.buf.clear();
            self.done = true;
            return SpecialTokenOutcome::Stop;
        }

        // `idx` is a byte offset produced by string search on `decoded`.
        let pre_special_token = &token_match.decoded[..token_match.idx];
        if !pre_special_token.is_empty() {
            let (o, _) = self.handle_token(self.mode, pre_special_token.as_bytes(), false);
            out.extend(o);
        }

        match new_mode {
            FilterMode::GroundedAnswer => {
                self.cur_text_index = 0;
                self.cur_text_byte_index = 0;
                self.cur_citation_byte_index = None;
                if self.stream_non_grounded_answer {
                    self.left_trimmed = true;
                }
            }
            FilterMode::ToolReason => {
                self.cur_text_index = 0;
                self.cur_text_byte_index = 0;
                self.cur_citation_byte_index = None;
                self.left_trimmed = true;
                self.right_trimmed = true;
            }
            _ => {}
        }

        let remove_len = pre_special_token.len() + token_match.sequence.len();
        self.buf.drain(..remove_len);
        self.mode = new_mode;
        SpecialTokenOutcome::Consumed
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
                if self.cofl_tool_action {
                    self.parse_cofl_actions(&s)
                } else {
                    self.parse_actions(&s)
                }
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
            FilterMode::GroundedAnswer | FilterMode::ToolReason => {
                // Citation / text index resets happen in `apply_special_token_match`
                // *after* pre-token bytes are flushed with `handle_token`, so
                // `cur_citation_byte_index` remains valid through the final
                // `GroundedAnswer` / `ToolReason` chunk before the delimiter.
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

    /// Classify decoded chunks by whether they emit content.
    pub fn classify_content_chunks(&mut self, token_strings: &[String]) -> Vec<bool> {
        token_strings
            .iter()
            .map(|token_str| self.write_decoded(token_str).content.is_some())
            .collect()
    }

    /// Process a complete model output string in one call.
    ///
    /// Unlike `process_full` which requires pre-tokenized chunks, this method
    /// takes the raw text and internally splits at special token boundaries,
    /// reducing the number of processing passes from `O(n_tokens)` to
    /// `O(n_special_tokens)`.
    pub fn process_full_text(&mut self, text: &str) -> FilterAggregatedResult {
        let tokens: Vec<String> = self.special_token_map.keys().cloned().collect();
        let mut all_outputs = Vec::new();
        let mut pos = 0;

        while pos < text.len() && !self.done {
            let remaining = &text[pos..];
            match find_partial(remaining, tokens.iter()) {
                PartialMatchResult::Full { idx, sequence } => {
                    let end = idx + sequence.len();
                    all_outputs.extend(self.write_text(&remaining.as_bytes()[..end]));
                    pos += end;
                }
                PartialMatchResult::Partial { .. } | PartialMatchResult::NoMatch => {
                    all_outputs.extend(self.write_text(remaining.as_bytes()));
                    pos = text.len();
                }
            }
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
///
/// Returns a ``Full`` match at the smallest byte index when any stop is
/// present, otherwise a ``Partial`` if the tail of ``s`` is a non-empty
/// prefix of some stop (so the caller can buffer until the next chunk
/// completes it), otherwise ``NoMatch``.
///
/// Iteration order of ``stops`` does not affect the output as long as no
/// stop is a prefix of another (true for all current callers).
pub(crate) fn find_partial<'a>(
    s: &str,
    stops: impl Iterator<Item = &'a String>,
) -> PartialMatchResult {
    let mut best_full: Option<(usize, String)> = None;
    let mut min_partial_idx: Option<usize> = None;

    for stop in stops {
        if let Some(idx) = s.find(stop) {
            if best_full.as_ref().is_none_or(|(cur_idx, _)| idx < *cur_idx) {
                best_full = Some((idx, stop.clone()));
            }
            continue;
        }
        // Otherwise look for a tail-prefix partial match.
        'inner: for i in 0..stop.len() {
            if !stop.is_char_boundary(stop.len() - i) {
                continue 'inner;
            }
            let suffix = &stop[..stop.len() - i];

            if s.ends_with(suffix) {
                let idx = s.len() - suffix.len();
                if min_partial_idx.is_none_or(|current_min_idx| current_min_idx > idx) {
                    min_partial_idx = Some(idx);
                }
                break;
            }
        }
    }

    if let Some((idx, sequence)) = best_full {
        PartialMatchResult::Full { idx, sequence }
    } else if let Some(idx) = min_partial_idx {
        PartialMatchResult::Partial { idx }
    } else {
        PartialMatchResult::NoMatch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::options::{FilterOptions, new_filter};
    use crate::parsing::types::{FilterCitation, FilterToolCallDelta, FilterToolParameter};

    #[test]
    fn test_find_partial() {
        let stops = vec!["<co: ".to_string(), "</co: ".to_string()];

        // Test full match
        match find_partial("hello <co: ", stops.iter()) {
            PartialMatchResult::Full { idx, sequence } => {
                assert_eq!(idx, 6);
                assert_eq!(sequence, "<co: ");
            }
            PartialMatchResult::NoMatch | PartialMatchResult::Partial { .. } => {
                panic!("expected full match")
            }
        }

        // Test partial match
        match find_partial("hello <c", stops.iter()) {
            PartialMatchResult::Partial { idx } => assert_eq!(idx, 6),
            PartialMatchResult::NoMatch | PartialMatchResult::Full { .. } => {
                panic!("expected partial match")
            }
        }

        // Test no match
        assert!(matches!(
            find_partial("hello world", stops.iter()),
            PartialMatchResult::NoMatch
        ));
    }

    /// Test: when several stop sequences match, ``find_partial`` must
    /// return the earliest match.
    #[test]
    fn test_find_partial_picks_earliest_full_match() {
        let stops = vec![
            "<|END_THINKING|>".to_string(),
            "<|START_ACTION|>".to_string(),
        ];
        match find_partial(" query1<|END_THINKING|><|START_ACTION|>", stops.iter()) {
            PartialMatchResult::Full { idx, sequence } => {
                assert_eq!(idx, 7);
                assert_eq!(sequence, "<|END_THINKING|>");
            }
            other => panic!("expected Full(end-thinking@7), got {other:?}"),
        }
    }

    #[test]
    fn test_find_partial_utf8() {
        // This test ensures we don't slice in the middle of a UTF-8 character (we used to panic here).
        let stops = vec!["RÈGLES".to_string()];
        match find_partial("ÈÈÈÈÈÈÈR", stops.iter()) {
            PartialMatchResult::Partial { idx } => assert_eq!(idx, 14),
            PartialMatchResult::NoMatch | PartialMatchResult::Full { .. } => {
                panic!("expected partial UTF-8 match")
            }
        }
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

    fn make_cmd3_filter() -> FilterImpl {
        new_filter(FilterOptions::default().cmd3())
    }

    fn make_cmd4_filter() -> FilterImpl {
        new_filter(FilterOptions::default().cmd4())
    }

    fn make_cmd4_no_tools_filter() -> FilterImpl {
        new_filter(FilterOptions::default().cmd4().no_tools())
    }

    fn make_cmd5_filter() -> FilterImpl {
        new_filter(FilterOptions::default().cmd5())
    }

    fn make_cmd5_no_tools_filter() -> FilterImpl {
        new_filter(FilterOptions::default().cmd5().no_tools())
    }

    #[test]
    fn test_process_full_text_cmd3_thinking_and_response() {
        let mut f = make_cmd3_filter();
        let text = "<|START_THINKING|>Let me think about this.\
                     <|END_THINKING|>\
                     <|START_RESPONSE|>Here is the answer.\
                     <|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("Let me think about this.".into()));
        assert_eq!(result.content, Some("Here is the answer.".into()));
    }

    /// Partial cmd3 citation spans two `process_full` chunks and completes in the same
    /// buffer as `<|END_THINKING|>`. Citation byte index must survive until that
    /// pre-token flush; resetting it too early duplicates the citation body in
    /// aggregated reasoning.
    ///
    /// Each `write_text` call handles at most one special-token match, so chunks
    /// are split so every delimiter sits in its own final `process_full` segment.
    #[test]
    fn test_streaming_partial_cmd3_citation_flush_before_end_thinking() {
        let mut f = make_cmd3_filter();
        let chunks = vec![
            "<|START_THINKING|>".to_string(),
            "pre <co>ci".to_string(),
            "tation</co: 0:[0]><|END_THINKING|>".to_string(),
            "<|START_RESPONSE|>out".to_string(),
            "<|END_RESPONSE|>".to_string(),
        ];
        let result = f.process_full(&chunks);
        assert_eq!(result.reasoning.as_deref(), Some("pre citation"));
        assert_eq!(result.content.as_deref(), Some("out"));
    }

    // test_citation_start_in_thinking_bug is a regression test for old behavior where citation state wasn't reset between modes causing parsing bugs.
    #[test]
    fn test_citation_start_in_thinking_bug() {
        let mut f = make_cmd3_filter();
        let text = "<|START_THINKING|>I will use some <co> tags to make citations<|END_THINKING|><|START_RESPONSE|>here is a <co>citation</co: 0:[0]>!!!<|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert_eq!(
            result.reasoning,
            Some("I will use some  tags to make citations".into())
        );
        assert_eq!(result.content, Some("here is a citation!!!".into()));
    }

    #[test]
    fn test_process_full_text_cmd4_thinking_and_response() {
        let mut f = make_cmd4_filter();
        let text = "<|START_THINKING|>Step 1: analyze.\
                     <|END_THINKING|>\
                     <|START_TEXT|>The result is 42.\
                     <|END_TEXT|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("Step 1: analyze.".into()));
        assert_eq!(result.content, Some("The result is 42.".into()));
    }

    #[test]
    fn test_process_full_text_cmd4_implicit_reasoning_then_text() {
        let mut f = make_cmd4_filter();
        let text = "Plan first.\
                     <|END_THINKING|>\
                     <|START_TEXT|>Final answer.\
                     <|END_TEXT|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("Plan first.".into()));
        assert_eq!(result.content, Some("Final answer.".into()));
    }

    #[test]
    fn test_process_full_text_cmd4_start_response_also_works() {
        let mut f = make_cmd4_filter();
        let text = "<|START_THINKING|>Think.\
                     <|END_THINKING|>\
                     <|START_RESPONSE|>Response text.\
                     <|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("Think.".into()));
        assert_eq!(result.content, Some("Response text.".into()));
    }

    #[test]
    fn test_process_full_text_response_only() {
        let mut f = make_cmd3_filter();
        let text = "<|START_RESPONSE|>Just a response.<|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert!(result.reasoning.is_none());
        assert_eq!(result.content, Some("Just a response.".into()));
    }

    #[test]
    fn test_process_full_text_thinking_only_no_response() {
        let mut f = make_cmd3_filter();
        let text = "<|START_THINKING|>I am thinking.<|END_THINKING|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("I am thinking.".into()));
        assert!(result.content.is_none());
    }

    #[test]
    fn test_process_full_text_plain_text_no_special_tokens() {
        let mut f = make_cmd3_filter();
        let text = "Hello world, no special tokens here.";
        let result = f.process_full_text(text);
        assert_eq!(
            result.content,
            Some("Hello world, no special tokens here.".into())
        );
        assert!(result.reasoning.is_none());
    }

    #[test]
    fn test_process_full_text_empty_string() {
        let mut f = make_cmd3_filter();
        let result = f.process_full_text("");
        assert!(result.content.is_none());
        assert!(result.reasoning.is_none());
        assert!(result.tool_calls.is_empty());
        assert!(result.citations.is_empty());
    }

    #[test]
    fn test_process_full_text_empty_thinking_block() {
        let mut f = make_cmd3_filter();
        let text = "<|START_THINKING|><|END_THINKING|><|START_RESPONSE|>Content.<|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert_eq!(result.content, Some("Content.".into()));
    }

    #[test]
    fn test_process_full_text_cmd4_empty_implicit_thinking_prefix() {
        let mut f = make_cmd4_filter();
        let text = "<|START_THINKING|><|END_THINKING|><|START_TEXT|>Content.<|END_TEXT|>";
        let result = f.process_full_text(text);
        assert!(result.reasoning.is_none());
        assert_eq!(result.content, Some("Content.".into()));
    }

    #[test]
    fn test_process_full_text_empty_response_block() {
        let mut f = make_cmd3_filter();
        let text = "<|START_THINKING|>Thinking.<|END_THINKING|>\
                     <|START_RESPONSE|><|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("Thinking.".into()));
    }

    #[test]
    fn test_process_full_text_utf8_multibyte() {
        let mut f = make_cmd4_no_tools_filter();
        let text = "<|START_THINKING|>Réflexion 🤔 über Ñoño\
                     <|END_THINKING|>\
                     <|START_RESPONSE|>Ответ: café ☕\
                     <|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("Réflexion 🤔 über Ñoño".into()));
        assert_eq!(result.content, Some("Ответ: café ☕".into()));
    }

    #[test]
    fn test_process_full_text_adjacent_special_tokens() {
        let mut f = make_cmd3_filter();
        let text = "<|START_THINKING|><|END_THINKING|>\
                     <|START_RESPONSE|><|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert!(result.reasoning.is_none());
        assert!(result.content.is_none());
    }

    #[test]
    fn test_process_full_text_long_content() {
        let mut f = make_cmd4_no_tools_filter();
        let filler = "word ".repeat(10_000);
        let text = format!(
            "<|START_THINKING|>{filler}<|END_THINKING|>\
             <|START_RESPONSE|>{filler}<|END_RESPONSE|>"
        );
        let result = f.process_full_text(&text);
        assert!(result.reasoning.is_some());
        assert!(result.content.is_some());
        let reasoning = result.reasoning.unwrap();
        let content = result.content.unwrap();
        assert!(reasoning.contains("word"));
        assert!(content.contains("word"));
        assert!(reasoning.len() > 40_000);
        assert!(content.len() > 40_000);
    }

    #[test]
    fn test_process_full_text_with_inclusive_stop() {
        let opts = FilterOptions::default()
            .cmd3()
            .with_inclusive_stops(vec!["STOP_HERE".to_string()]);
        let mut f = new_filter(opts);
        let text = "<|START_RESPONSE|>Before stop STOP_HERE After stop<|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert!(result.content.is_some());
        let content = result.content.unwrap();
        assert!(content.contains("Before stop"));
        assert!(content.contains("STOP_HERE"));
        assert!(!content.contains("After stop"));
    }

    #[test]
    fn test_process_full_text_with_exclusive_stop() {
        let opts = FilterOptions::default()
            .cmd3()
            .with_exclusive_stops(vec!["STOP_HERE".to_string()]);
        let mut f = new_filter(opts);
        let text = "<|START_RESPONSE|>Before stop STOP_HERE After stop<|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert!(result.content.is_some());
        let content = result.content.unwrap();
        assert!(content.contains("Before stop"));
        assert!(!content.contains("STOP_HERE"));
        assert!(!content.contains("After stop"));
    }

    #[test]
    fn test_process_full_text_matches_process_full_simple() {
        let text = "<|START_THINKING|>Think hard.\
                     <|END_THINKING|>\
                     <|START_RESPONSE|>The answer is 7.\
                     <|END_RESPONSE|>";

        let chunks: Vec<String> = text.chars().map(|c| c.to_string()).collect();

        let mut f1 = make_cmd3_filter();
        let result_full_text = f1.process_full_text(text);

        let mut f2 = make_cmd3_filter();
        let result_full = f2.process_full(&chunks);

        assert_eq!(result_full_text.reasoning, result_full.reasoning);
        assert_eq!(result_full_text.content, result_full.content);
        assert_eq!(
            result_full_text.tool_calls.len(),
            result_full.tool_calls.len()
        );
        assert_eq!(
            result_full_text.citations.len(),
            result_full.citations.len()
        );
    }

    #[test]
    fn test_process_full_text_matches_process_full_cmd4() {
        let text = "<|START_THINKING|>Plan: step 1, step 2.\
                     <|END_THINKING|>\
                     <|START_TEXT|>Final result here.\
                     <|END_TEXT|>";

        let chunks: Vec<String> = text.chars().map(|c| c.to_string()).collect();

        let mut f1 = make_cmd4_no_tools_filter();
        let result_full_text = f1.process_full_text(text);

        let mut f2 = make_cmd4_no_tools_filter();
        let result_full = f2.process_full(&chunks);

        assert_eq!(result_full_text.reasoning, result_full.reasoning);
        assert_eq!(result_full_text.content, result_full.content);
    }

    #[test]
    fn test_process_full_text_matches_process_full_plain_text() {
        let text = "Just plain text without any markers.";

        let chunks: Vec<String> = text.chars().map(|c| c.to_string()).collect();

        let mut f1 = make_cmd3_filter();
        let result_full_text = f1.process_full_text(text);

        let mut f2 = make_cmd3_filter();
        let result_full = f2.process_full(&chunks);

        assert_eq!(result_full_text.content, result_full.content);
        assert_eq!(result_full_text.reasoning, result_full.reasoning);
    }

    #[test]
    fn test_process_full_text_matches_process_full_utf8() {
        let text = "<|START_THINKING|>日本語テスト\
                     <|END_THINKING|>\
                     <|START_RESPONSE|>中文回答\
                     <|END_RESPONSE|>";

        let chunks: Vec<String> = text.chars().map(|c| c.to_string()).collect();

        let mut f1 = make_cmd3_filter();
        let result_full_text = f1.process_full_text(text);

        let mut f2 = make_cmd3_filter();
        let result_full = f2.process_full(&chunks);

        assert_eq!(result_full_text.reasoning, result_full.reasoning);
        assert_eq!(result_full_text.content, result_full.content);
    }

    #[test]
    fn test_process_full_text_with_tool_action() {
        let opts = FilterOptions::default().cmd4().stream_tool_actions();
        let mut f = new_filter(opts);
        let text = "<|START_THINKING|>I should search.\
                     <|END_THINKING|>\
                     <|START_ACTION|>\n[{\"tool_call_id\": \"call_0\", \"tool_name\": \"web_search\", \"parameters\": {\"query\": \"test\"}}]\
                     <|END_ACTION|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("I should search.".into()));
        assert!(!result.tool_calls.is_empty());
        assert_eq!(result.tool_calls[0].name, "web_search");
    }

    #[test]
    fn test_process_full_text_cmd4_implicit_reasoning_then_tool_action() {
        let opts = FilterOptions::default().cmd4().stream_tool_actions();
        let mut f = new_filter(opts);
        let text = "I should search.\
                     <|END_THINKING|>\
                     <|START_ACTION|>\n[{\"tool_call_id\": \"call_0\", \"tool_name\": \"web_search\", \"parameters\": {\"query\": \"test\"}}]\
                     <|END_ACTION|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("I should search.".into()));
        assert!(!result.tool_calls.is_empty());
        assert_eq!(result.tool_calls[0].name, "web_search");
    }

    #[test]
    fn test_process_full_text_text_before_first_special_token() {
        let mut f = make_cmd3_filter();
        let text = "Preamble text <|START_RESPONSE|>Response.<|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert!(result.content.is_some());
        let content = result.content.unwrap();
        assert!(content.contains("Preamble text"));
        assert!(content.contains("Response."));
    }

    #[test]
    fn test_process_full_text_text_after_end_response() {
        let mut f = make_cmd3_filter();
        let text = "<|START_RESPONSE|>Content.<|END_RESPONSE|>Trailing garbage";
        let result = f.process_full_text(text);
        assert_eq!(result.content, Some("Content.".into()));
    }

    #[test]
    fn test_process_full_text_repeated_thinking_blocks() {
        let mut f = make_cmd3_filter();
        let text = "<|START_THINKING|>First thought.<|END_THINKING|>\
                     <|START_RESPONSE|>Middle answer.<|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("First thought.".into()));
        assert_eq!(result.content, Some("Middle answer.".into()));
    }

    #[test]
    fn test_process_full_text_special_token_like_substring() {
        let mut f = make_cmd4_no_tools_filter();
        let text = "<|START_RESPONSE|>The tag <|NOT_A_TOKEN|> is not special.<|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert!(result.content.is_some());
        let content = result.content.unwrap();
        assert!(content.contains("<|NOT_A_TOKEN|>"));
    }

    #[test]
    fn test_process_full_text_citations_in_response() {
        let mut f = make_cmd3_filter();
        let text = "<|START_RESPONSE|>The sky is <co: 0>blue</co: 0>.<|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert!(result.content.is_some());
        let content = result.content.unwrap();
        assert!(content.contains("blue"));
        assert!(!result.citations.is_empty());
        assert_eq!(result.citations[0].text, "blue");
    }

    #[test]
    fn test_process_full_text_no_tools_mode() {
        let mut f = make_cmd4_no_tools_filter();
        let text = "<|START_THINKING|>Reasoning.\
                     <|END_THINKING|>\
                     <|START_RESPONSE|>Answer.\
                     <|END_RESPONSE|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("Reasoning.".into()));
        assert_eq!(result.content, Some("Answer.".into()));
        assert!(result.tool_calls.is_empty());
    }

    #[test]
    fn test_process_full_text_sets_done_flag() {
        let mut f = make_cmd3_filter();
        let text = "<|START_RESPONSE|>Hello.<|END_RESPONSE|>";
        let _ = f.process_full_text(text);
        assert!(f.done);

        let result = f.write_text(b"More text");
        assert!(result.is_empty());
    }

    #[test]
    fn test_classify_content_chunks_marks_only_content_after_reasoning() {
        let mut f = make_cmd4_no_tools_filter();
        let chunks = vec![
            "<|START_THINKING|>".to_string(),
            "Step 1".to_string(),
            "<|END_THINKING|>".to_string(),
            "<|START_TEXT|>".to_string(),
            "Final".to_string(),
            " answer".to_string(),
            "<|END_TEXT|>".to_string(),
        ];

        let content_mask = f.classify_content_chunks(&chunks);

        assert_eq!(
            content_mask,
            vec![false, false, false, false, true, true, false]
        );
    }

    #[test]
    fn test_classify_content_chunks_marks_transition_chunk_with_content() {
        let mut f = make_cmd3_filter();
        let chunks = vec![
            "<|START_THINKING|>".to_string(),
            "Thinking".to_string(),
            "<|END_THINKING|>Answer".to_string(),
            "<|END_RESPONSE|>".to_string(),
        ];

        let content_mask = f.classify_content_chunks(&chunks);

        assert_eq!(content_mask, vec![false, false, true, false]);
    }

    /// Regression test for the speculative-decoding leak: when several
    /// accepted tokens are decoded as a single chunk, the buffer may
    /// contain two complete special-token boundaries (e.g.
    /// ``<|END_THINKING|><|START_ACTION|>``). The streaming path must
    /// consume both, not just the first otherwise the second marker
    /// leaks out as raw text in the new mode and the downstream parser
    /// fails to recognize the tool-call boundary.
    #[test]
    fn test_write_decoded_handles_multiple_special_tokens_in_one_chunk() {
        let opts = FilterOptions::default().cmd4().stream_tool_actions();
        let mut f = new_filter(opts);

        f.write_decoded("<|START_THINKING|>");
        f.write_decoded(" think ");
        // end-of-thinking + start-of-action arrive together.
        // The second marker must be consumed, not emitted as text.
        let r = f.write_decoded("<|END_THINKING|><|START_ACTION|>");
        assert!(
            r.content.is_none(),
            "second special token leaked as content: {:?}",
            r.content,
        );
        assert!(r.tool_calls.is_empty());

        // The remaining tool-call JSON should now be parsed as a tool call,
        // not emitted as content.
        let r = f.write_decoded(
            r#"[{"tool_call_id": "0", "tool_name": "foo", "parameters": {"q": "x"}}]"#,
        );
        assert!(
            r.content.is_none(),
            "tool-call args leaked as content: {:?}",
            r.content
        );
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].name, "foo");
        assert_eq!(r.tool_calls[0].id, "0");
    }

    /// Single-chunk reasoning-end + tool-action-start + partial tool args
    #[test]
    fn test_write_decoded_multi_special_token_with_partial_action_body() {
        let opts = FilterOptions::default().cmd4().stream_tool_actions();
        let mut f = new_filter(opts);

        f.write_decoded("<|START_THINKING|>");
        f.write_decoded(" think");
        // All three boundaries plus a partial JSON body in one chunk.
        let r = f.write_decoded(
            r#"<|END_THINKING|><|START_ACTION|>[{"tool_call_id": "0", "tool_name": "foo", "parameters": {"q": "x"}}]"#,
        );

        assert!(
            r.content.is_none(),
            "special token / tool args leaked as content: {:?}",
            r.content,
        );
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].name, "foo");
        assert_eq!(r.tool_calls[0].id, "0");
    }

    /// When a chunk ends mid-special-token (e.g. ``<|START_AC``), the
    /// outputs accumulated from earlier full special tokens in the same
    /// chunk must still be returned. They must not be silently dropped
    /// while waiting for the partial token to complete.
    #[test]
    fn test_write_decoded_partial_after_full_keeps_earlier_outputs() {
        let opts = FilterOptions::default().cmd4().stream_tool_actions();
        let mut f = new_filter(opts);

        f.write_decoded("<|START_THINKING|>");
        f.write_decoded(" think");
        // End-of-thinking followed by an incomplete next special token.
        // The first must transition the mode; the partial bytes stay
        // buffered for the next call to complete.
        let r = f.write_decoded("<|END_THINKING|><|START_AC");
        assert!(r.content.is_none());
        assert!(r.tool_calls.is_empty());

        // Next chunk completes the previously partial special token, then
        // delivers the tool-call body. No content should be emitted.
        let r = f.write_decoded(
            r#"TION|>[{"tool_call_id": "1", "tool_name": "bar", "parameters": {}}]"#,
        );
        assert!(r.content.is_none(), "leaked content: {:?}", r.content);
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].name, "bar");
        assert_eq!(r.tool_calls[0].id, "1");
    }

    #[test]
    fn test_classify_content_chunks_excludes_tool_action_chunks() {
        let opts = FilterOptions::default().cmd4().stream_tool_actions();
        let mut f = new_filter(opts);
        let chunks = vec![
            "<|START_THINKING|>".to_string(),
            "Need a tool.".to_string(),
            "<|END_THINKING|>".to_string(),
            "<|START_ACTION|>".to_string(),
            r#"[{"tool_call_id":"call_0","tool_name":"web_search","parameters":{"query":"weather"}}]"#
                .to_string(),
            "<|END_ACTION|>".to_string(),
            "<|START_TEXT|>".to_string(),
            "Sunny.".to_string(),
            "<|END_TEXT|>".to_string(),
        ];

        let content_mask = f.classify_content_chunks(&chunks);

        assert_eq!(
            content_mask,
            vec![false, false, false, false, false, false, false, true, false]
        );
    }

    #[test]
    fn test_process_full_text_cmd5_thinking_and_text() {
        let mut f = make_cmd5_no_tools_filter();
        let text = "<|START_THINKING|>Let me think.\
                     <|END_THINKING|>\
                     <|START_TEXT|>Here is the answer.\
                     <|END_TEXT|>";
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("Let me think.".into()));
        assert_eq!(result.content, Some("Here is the answer.".into()));
    }

    #[test]
    fn test_process_full_text_cmd5_tool_call_with_string_param() {
        let mut f = make_cmd5_filter();
        let text = r#"<|START_THINKING|>I should search.<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="call_0" name="web_search"><cofl:tool_param name="query" string="true">test</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("I should search.".into()));
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_0");
        assert_eq!(result.tool_calls[0].name, "web_search");
        assert_eq!(result.tool_calls[0].arguments, r#"{"query": "test"}"#);
    }

    #[test]
    fn test_process_full_text_cmd5_tool_call_mixed_param_types() {
        let mut f = make_cmd5_filter();
        let text = r#"<|START_THINKING|>thinking<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="DeleteReminder"><cofl:tool_param name="reminder_id" string="true">12-abc</cofl:tool_param><cofl:tool_param name="force" string="false">true</cofl:tool_param><cofl:tool_param name="limit" string="false">3</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "DeleteReminder");
        assert_eq!(
            result.tool_calls[0].arguments,
            r#"{"reminder_id": "12-abc", "force": true, "limit": 3}"#
        );
    }

    /// Nested `string="false"` JSON (array of objects with shell commands,
    /// embedded quotes, backslashes, and newlines) must round-trip through
    /// the cofl parser as valid tool-call arguments.
    #[test]
    fn test_process_full_text_cmd5_execute_command_nested_commands() {
        let commands = serde_json::json!([
            {
                "cmd": "grep -n \"with open(csv_path\" -n /app/validate.py\n",
                "time": 0.1
            },
            {
                "cmd": "sed -i \"s/writer = csv\\.writer(f)/writer = csv.writer(f, lineterminator='\\n')/\" /app/validate.py\n",
                "time": 0.1
            },
            {
                "cmd": "grep -n \"writer = csv\" -n /app/validate.py\n",
                "time": 0.1
            }
        ]);
        let expected_args = serde_json::json!({ "commands": commands });
        let commands_wire = serde_json::to_string(&commands).expect("wire JSON");

        let text = format!(
            r#"<|START_THINKING|>run commands<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="17" name="execute_command"><cofl:tool_param name="commands" string="false">{commands_wire}</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#
        );

        let mut f = make_cmd5_filter();
        let result = f.process_full_text(&text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "17");
        assert_eq!(result.tool_calls[0].name, "execute_command");

        let parsed: serde_json::Value =
            serde_json::from_str(&result.tool_calls[0].arguments).expect("valid JSON");
        assert_eq!(parsed, expected_args);

        // Streaming one character at a time must produce the same arguments.
        let chunks: Vec<String> = text.chars().map(|c| c.to_string()).collect();
        let mut f2 = make_cmd5_filter();
        let streamed = f2.process_full(&chunks);
        assert_eq!(streamed.tool_calls.len(), 1);
        assert_eq!(
            streamed.tool_calls[0].arguments,
            result.tool_calls[0].arguments
        );
    }

    #[test]
    fn test_process_full_text_cmd5_multiple_tool_calls() {
        let mut f = make_cmd5_filter();
        let text = r#"<|START_THINKING|>parallel<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="GetReminders"></cofl:tool_call><cofl:tool_call id="1" name="GetTodos"><cofl:tool_param name="filter" string="true">open</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.tool_calls.len(), 2);
        assert_eq!(result.tool_calls[0].id, "0");
        assert_eq!(result.tool_calls[0].name, "GetReminders");
        assert_eq!(result.tool_calls[0].arguments, "{}");
        assert_eq!(result.tool_calls[1].id, "1");
        assert_eq!(result.tool_calls[1].name, "GetTodos");
        assert_eq!(result.tool_calls[1].arguments, r#"{"filter": "open"}"#);
    }

    /// Rainbow emoji (U+1F308) is a 4-byte UTF-8 sequence. It must round-trip
    /// through both `string="true"` parameters (where the body is JSON-escaped
    /// per character) and `string="false"` parameters (where the body is
    /// emitted verbatim as a JSON literal). Streaming the same input one char
    /// at a time must produce the same aggregated arguments.
    #[test]
    fn test_process_full_text_cmd5_tool_call_with_emoji_params() {
        let text = r#"<|START_THINKING|>think<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="send"><cofl:tool_param name="message" string="true">Hello 🌈 world! ☕</cofl:tool_param><cofl:tool_param name="tags" string="false">["🌈", "🦄"]</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;

        let mut f = make_cmd5_filter();
        let result = f.process_full_text(text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "0");
        assert_eq!(result.tool_calls[0].name, "send");

        let parsed: serde_json::Value =
            serde_json::from_str(&result.tool_calls[0].arguments).expect("valid JSON");
        assert_eq!(parsed["message"], "Hello 🌈 world! ☕");
        assert_eq!(parsed["tags"], serde_json::json!(["🌈", "🦄"]));

        // Streaming character-by-character (which splits the buffer at every
        // codepoint, including ones in the middle of the cofl tag bodies)
        // must produce the same aggregated arguments. This guards against
        // the per-chunk JSON escaping in `emit_cofl_param_value_chunk`
        // accidentally splitting a multi-byte UTF-8 emoji.
        let chunks: Vec<String> = text.chars().map(|c| c.to_string()).collect();
        let mut f2 = make_cmd5_filter();
        let streamed = f2.process_full(&chunks);
        assert_eq!(
            streamed.tool_calls[0].arguments,
            result.tool_calls[0].arguments
        );
    }

    #[test]
    fn test_process_full_text_cmd5_processed_params_mode() {
        // When stream_processed_params is enabled the raw_param_delta
        // stream is suppressed and the structured `processed_params` are
        // populated instead, mirroring action_filter behaviour.
        let opts = FilterOptions::default().cmd5().stream_processed_params();
        let mut f = new_filter(opts);
        let text = r#"<|START_THINKING|>thinking<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="DeleteReminder"><cofl:tool_param name="reminder_id" string="true">12-abc</cofl:tool_param><cofl:tool_param name="force" string="false">true</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "DeleteReminder");
        // raw arguments stream is empty in processed mode.
        assert_eq!(result.tool_calls[0].arguments, "");
        // processed params should reflect the JSON-shaped values.
        let params = &result.tool_calls[0].processed_params;
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "reminder_id");
        assert_eq!(params[0].value_delta, "\"12-abc\"");
        assert_eq!(params[1].name, "force");
        assert_eq!(params[1].value_delta, "true");
    }

    #[test]
    fn test_process_full_text_cmd5_matches_streaming_process_full() {
        let text = r#"<|START_THINKING|>think.<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="search"><cofl:tool_param name="q" string="true">hello</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;
        let chunks: Vec<String> = text.chars().map(|c| c.to_string()).collect();

        let mut f1 = make_cmd5_filter();
        let r_full_text = f1.process_full_text(text);
        let mut f2 = make_cmd5_filter();
        let r_full = f2.process_full(&chunks);

        assert_eq!(r_full_text.reasoning, r_full.reasoning);
        assert_eq!(r_full_text.tool_calls.len(), r_full.tool_calls.len());
        assert_eq!(
            r_full_text.tool_calls[0].arguments,
            r_full.tool_calls[0].arguments
        );
        assert_eq!(r_full_text.tool_calls[0].name, r_full.tool_calls[0].name);
        assert_eq!(r_full_text.tool_calls[0].id, r_full.tool_calls[0].id);
    }

    /// An empty `string="true"` parameter body must still produce a valid
    /// JSON string (`""`), with both the opening and closing `"` emitted.
    #[test]
    fn test_process_full_text_cmd5_empty_string_param_value() {
        let mut f = make_cmd5_filter();
        let text = r#"<|START_THINKING|>think<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="set_note"><cofl:tool_param name="note" string="true"></cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].arguments, r#"{"note": ""}"#);
        let parsed: serde_json::Value =
            serde_json::from_str(&result.tool_calls[0].arguments).expect("valid JSON");
        assert_eq!(parsed["note"], "");
    }

    /// A `string="true"` parameter body with XML-entity escaped `<` and `>`
    /// (as produced by the cmd5 template's `xml_text` macro) must decode to
    /// the original characters in the tool-call arguments.
    #[test]
    fn test_process_full_text_cmd5_string_param_value_with_angle_brackets() {
        let mut f = make_cmd5_filter();
        let snippet = "if (a < b) { return <T>(); }";
        let wire = "if (a &lt; b) { return &lt;T&gt;(); }";
        let text = format!(
            r#"<|START_THINKING|>think<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="run_code"><cofl:tool_param name="snippet" string="true">{wire}</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#
        );
        let result = f.process_full_text(&text);
        assert_eq!(result.tool_calls.len(), 1);
        let parsed: serde_json::Value =
            serde_json::from_str(&result.tool_calls[0].arguments).expect("valid JSON");
        assert_eq!(parsed["snippet"], snippet);
    }

    /// Round-trip the no-escape wire format produced by the `cmd5-no-escape`
    /// template (unescaped bodies, escaped attributes).
    #[test]
    fn test_process_full_text_cmd5_no_xml_text_decode() {
        let opts = FilterOptions::default().cmd5().cofl_no_xml_text_decode();
        let mut f = new_filter(opts);
        let text = r#"<|START_THINKING|>I'll call the tool now.<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="run&lt;cmd&gt;&amp;tool"><cofl:tool_param name="str_param" string="true">value with <tag> & "quotes" & &amp;</cofl:tool_param><cofl:tool_param name="num_param" string="false">42</cofl:tool_param><cofl:tool_param name="list_param" string="false">["a<b", "c&d"]</cofl:tool_param><cofl:tool_param name="param&lt;&gt;&amp;name" string="true">attr test</cofl:tool_param><cofl:tool_param name="nested" string="false">{"key<1>": "val>2"}</cofl:tool_param><cofl:tool_param name="filters" string="false">{"artist": "The \"Sudan\" Ensemble", "note": "line1\nline2"}</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "run<cmd>&tool");

        let parsed: serde_json::Value =
            serde_json::from_str(&result.tool_calls[0].arguments).expect("valid JSON");
        assert_eq!(parsed["str_param"], "value with <tag> & \"quotes\" & &amp;");
        assert_eq!(parsed["num_param"], 42);
        assert_eq!(parsed["list_param"], serde_json::json!(["a<b", "c&d"]));
        assert_eq!(parsed["param<>&name"], "attr test");
        assert_eq!(parsed["nested"], serde_json::json!({"key<1>": "val>2"}));
        assert_eq!(
            parsed["filters"],
            serde_json::json!({"artist": "The \"Sudan\" Ensemble", "note": "line1\nline2"})
        );
    }

    /// Round-trip the XML-entity escaping used by the cmd5 template, mirroring
    /// `tests/templating/jinja/cmd5/xml_escaping`.
    #[test]
    fn test_process_full_text_cmd5_xml_entity_escaping() {
        let mut f = make_cmd5_filter();
        let text = r#"<|START_THINKING|>I'll call the tool now.<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="run&lt;cmd&gt;&amp;tool"><cofl:tool_param name="str_param" string="true">value with &lt;tag&gt; &amp; "quotes"</cofl:tool_param><cofl:tool_param name="num_param" string="false">42</cofl:tool_param><cofl:tool_param name="bool_param" string="false">true</cofl:tool_param><cofl:tool_param name="list_param" string="false">["a&lt;b", "c&amp;d"]</cofl:tool_param><cofl:tool_param name="param&lt;&gt;&amp;name" string="true">attr test</cofl:tool_param><cofl:tool_param name="nested" string="false">{"key&lt;1&gt;": "val&gt;2"}</cofl:tool_param><cofl:tool_param name="filters" string="false">{"artist": "The \"Sudan\" Ensemble", "note": "line1\nline2"}</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "0");
        assert_eq!(result.tool_calls[0].name, "run<cmd>&tool");

        let parsed: serde_json::Value =
            serde_json::from_str(&result.tool_calls[0].arguments).expect("valid JSON");
        assert_eq!(parsed["str_param"], "value with <tag> & \"quotes\"");
        assert_eq!(parsed["num_param"], 42);
        assert_eq!(parsed["bool_param"], true);
        assert_eq!(parsed["list_param"], serde_json::json!(["a<b", "c&d"]));
        assert_eq!(parsed["param<>&name"], "attr test");
        assert_eq!(parsed["nested"], serde_json::json!({"key<1>": "val>2"}));
        assert_eq!(
            parsed["filters"],
            serde_json::json!({"artist": "The \"Sudan\" Ensemble", "note": "line1\nline2"})
        );
    }

    /// Round-trip nested-xml cofl tool calls from the cmd5-nested-xml template.
    #[test]
    fn test_process_full_text_cmd5_nested_xml() {
        let opts = FilterOptions::default().cmd5().cofl_nested_xml();
        let mut f = new_filter(opts);
        let text = r#"<|START_THINKING|>searching<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="search &quot;web&quot;"><cofl:value name="query" type="raw">echo "Hello" >> foo.txt && exit</cofl:value><cofl:value name="limit" type="json">3</cofl:value><cofl:value name="float example" type="json">3.14</cofl:value><cofl:value name="filters" type="dict"><cofl:value name="fresh" type="json">true</cofl:value><cofl:value name="tags" type="list"><cofl:value type="raw">music</cofl:value><cofl:value type="raw">Sudan</cofl:value></cofl:value></cofl:value><cofl:value name="missing" type="json">null</cofl:value><cofl:value name="empty_dict" type="dict"></cofl:value><cofl:value name="empty_list" type="list"></cofl:value></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, r#"search "web""#);

        let parsed: serde_json::Value =
            serde_json::from_str(&result.tool_calls[0].arguments).expect("valid JSON");
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
    fn test_process_full_text_cmd5_nested_xml_processed_params() {
        let opts = FilterOptions::default()
            .cmd5()
            .cofl_nested_xml()
            .stream_processed_params();
        let mut f = new_filter(opts);
        let text = r#"<|START_THINKING|>searching<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="0" name="search"><cofl:value name="query" type="raw">hello</cofl:value><cofl:value name="filters" type="dict"><cofl:value name="fresh" type="json">true</cofl:value><cofl:value name="tags" type="list"><cofl:value type="raw">music</cofl:value></cofl:value></cofl:value></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "search");
        assert_eq!(result.tool_calls[0].arguments, "");

        let params = &result.tool_calls[0].processed_params;
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "query");
        assert_eq!(params[0].value_delta, "\"hello\"");
        assert_eq!(params[1].name, "filters");
        let filters: serde_json::Value =
            serde_json::from_str(&params[1].value_delta).expect("valid JSON");
        assert_eq!(
            filters,
            serde_json::json!({"fresh": true, "tags": ["music"]})
        );
    }

    /// cmd5 generation prompts include `<|START_THINKING|>`, so the
    /// reasoning block can be implicit (no explicit start token) and the
    /// stream may begin directly with reasoning text terminated by
    /// `<|END_THINKING|>`. Mirrors the cmd4 implicit-reasoning test.
    #[test]
    fn test_process_full_text_cmd5_implicit_reasoning_then_tool_call() {
        let mut f = make_cmd5_filter();
        let text = r#"Plan first.<|END_THINKING|><cofl:tool_calls><cofl:tool_call id="call_0" name="web_search"><cofl:tool_param name="query" string="true">test</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("Plan first.".into()));
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_0");
        assert_eq!(result.tool_calls[0].name, "web_search");
        assert_eq!(result.tool_calls[0].arguments, r#"{"query": "test"}"#);
    }

    /// With `no_tools()` the `<cofl:tool_calls>` / `</cofl:tool_calls>`
    /// wrappers are removed from the special-token map, so cofl markup
    /// must pass through as plain content rather than transitioning into
    /// `ToolAction` mode.
    ///
    /// `no_tools` is for vllm where parsing happens in two phases.
    /// In the first phase (reasoning extraction) the tool calls
    /// must be passed through as plain text so the second phase
    // (tool call parsing) can parse them regularly.
    #[test]
    fn test_process_full_text_cmd5_no_tools_treats_cofl_as_plain_text() {
        let mut f = make_cmd5_no_tools_filter();
        let cofl = r#"<cofl:tool_calls><cofl:tool_call id="0" name="x"></cofl:tool_call></cofl:tool_calls>"#;
        let text = format!("<|START_THINKING|>think<|END_THINKING|>{cofl}");
        let result = f.process_full_text(&text);
        assert_eq!(result.reasoning, Some("think".into()));
        let content = result.content.expect("expected cofl markup as content");
        assert!(
            content.contains("<cofl:tool_calls>"),
            "opening wrapper missing from content: {content:?}",
        );
        assert!(
            content.contains(r#"<cofl:tool_call id="0" name="x">"#),
            "inner tool_call markup missing from content: {content:?}",
        );
        assert!(
            content.contains("</cofl:tool_calls>"),
            "closing wrapper missing from content: {content:?}",
        );
        assert!(result.tool_calls.is_empty());
    }

    /// An empty `<cofl:tool_calls></cofl:tool_calls>` block (no inner tool
    /// calls) must produce zero tool calls and no stray content. The body
    /// dispatcher in `BeforeToolCall` mode never sees any input here
    /// because the closing wrapper is consumed by the surrounding
    /// special-token state machine.
    #[test]
    fn test_process_full_text_cmd5_empty_tool_calls_block() {
        let mut f = make_cmd5_filter();
        let text = r#"<|START_THINKING|>think<|END_THINKING|><cofl:tool_calls></cofl:tool_calls>"#;
        let result = f.process_full_text(text);
        assert_eq!(result.reasoning, Some("think".into()));
        assert!(result.tool_calls.is_empty());
        assert!(result.content.is_none());
    }

    /// Test when ``handle_special_token`` rejects a match via the
    /// ``not_special`` rule (e.g. ``Answer:`` seen while already in
    /// ``GroundedAnswer`` mode), ``apply_special_token_match`` returns false
    /// without mutating the buffer or the mode. The streaming loop must
    /// detect this and fall through to plain buffer processing otherwise
    /// ``detect_special_token`` keeps returning the same ``Found`` result on
    /// every iteration loops forever.
    #[test]
    fn test_write_decoded_rejected_special_token_does_not_loop() {
        let mut f = new_filter(FilterOptions::default().handle_rag());

        f.write_decoded("Grounded answer:");
        // ``Answer:`` here is *not* a valid transition (already in
        // GroundedAnswer); it must be emitted as part of the grounded text.
        let r = f.write_decoded(" the Answer: is 42.");
        let content = r.content.unwrap_or_default();
        assert!(
            content.contains("Answer:"),
            "expected the rejected ``Answer:`` to be emitted as content, got {content:?}",
        );
    }
}
