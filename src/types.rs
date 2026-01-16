//! Type definitions for the Melody parsing library
//!
//! This module contains all the core data structures used throughout the library,
//! including output types, configuration enums, and helper structures.

#[cfg(feature = "python_ffi")]
use pyo3::prelude::*;
use serde::Deserialize;

/// Token IDs paired with their log probabilities.
///
/// This structure is used to track both the token identifiers and their associated
/// log probability scores from the language model. Log probabilities are useful for
/// understanding model confidence and implementing features like token filtering.
///
/// # Examples
///
/// ```rust
/// use cohere_melody::TokenIDsWithLogProb;
///
/// let mut logprobs = TokenIDsWithLogProb::new();
/// assert!(logprobs.token_ids.is_empty());
///
/// let other = TokenIDsWithLogProb {
///     token_ids: vec![1, 2, 3],
///     logprobs: vec![-0.1, -0.2, -0.3],
/// };
/// logprobs.append(other);
/// assert_eq!(logprobs.token_ids.len(), 3);
/// ```
#[cfg_attr(feature = "python_ffi", pyclass(get_all))]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct TokenIDsWithLogProb {
    /// Token IDs from the model's vocabulary
    pub token_ids: Vec<u32>,
    /// Log probability scores for each token (same length as `token_ids`)
    pub logprobs: Vec<f32>,
}

impl TokenIDsWithLogProb {
    /// Creates a new empty `TokenIDsWithLogProb` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::TokenIDsWithLogProb;
    ///
    /// let logprobs = TokenIDsWithLogProb::new();
    /// assert!(logprobs.token_ids.is_empty());
    /// assert!(logprobs.logprobs.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            token_ids: Vec::new(),
            logprobs: Vec::new(),
        }
    }

    /// Appends another `TokenIDsWithLogProb` to this one, extending both vectors.
    pub fn append(&mut self, other: TokenIDsWithLogProb) {
        self.token_ids.extend(other.token_ids);
        self.logprobs.extend(other.logprobs);
    }
}

/// A parsed output chunk from the streaming filter.
///
/// This is the primary output structure returned when processing tokens. Each call to
/// `write_decoded` may produce zero or more `FilterOutput` instances, depending on
/// what structured content is found in the token stream.
///
/// # Fields
///
/// - `text`: Plain text content extracted from the stream
/// - `logprobs`: Token IDs and log probabilities for this chunk
/// - `search_query`: Incremental search query updates (if parsing search queries)
/// - `citations`: Parsed citations found in this chunk
/// - `tool_call_delta`: Incremental tool call updates (if parsing tool calls)
/// - `is_post_answer`: True if this is content after an "Answer:" marker
/// - `is_reasoning`: True if this is content from a thinking/reasoning block
///
/// # Examples
///
/// ```rust
/// use cohere_melody::{FilterOutput, FilterCitation};
///
/// let output = FilterOutput {
///     text: "Hello world".to_string(),
///     citations: vec![],
///     ..Default::default()
/// };
/// assert_eq!(output.text, "Hello world");
/// ```
#[cfg_attr(feature = "python_ffi", pyclass(get_all))]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterOutput {
    /// Plain text content extracted from the token stream
    pub text: String,
    /// Token IDs and log probabilities for this output chunk
    pub logprobs: TokenIDsWithLogProb,
    /// Incremental search query delta (if in search query mode)
    pub search_query: Option<FilterSearchQueryDelta>,
    /// Citations parsed from this chunk (may be empty)
    pub citations: Vec<FilterCitation>,
    /// Incremental tool call delta (if in tool action mode)
    pub tool_call_delta: Option<FilterToolCallDelta>,
    /// True if this content appears after an "Answer:" marker
    pub is_post_answer: bool,
    /// True if this content is from a thinking/reasoning block
    pub is_reasoning: bool,
}

/// An incremental update to a search query being parsed.
///
/// When parsing search queries from the model output, this structure represents
/// a partial update to a specific query. Multiple deltas with the same index
/// should be concatenated to build the full query text.
///
/// # Examples
///
/// ```rust
/// use cohere_melody::FilterSearchQueryDelta;
///
/// let delta = FilterSearchQueryDelta {
///     index: 0,
///     text: "climate".to_string(),
/// };
/// assert_eq!(delta.index, 0);
/// ```
#[cfg_attr(feature = "python_ffi", pyclass(get_all))]
#[derive(Debug, Clone, PartialEq)]
pub struct FilterSearchQueryDelta {
    /// Index of the search query (0-based, for multi-query scenarios)
    pub index: usize,
    /// Incremental text for this search query
    pub text: String,
}

/// An incremental update to a tool call being parsed.
///
/// Tool calls are parsed incrementally as tokens arrive. This structure represents
/// a partial update that should be combined with previous updates to reconstruct
/// the full tool call.
///
/// # Fields
///
/// - `index`: Tool call index (for tracking multiple simultaneous tool calls)
/// - `id`: Tool call ID (may be empty on early updates)
/// - `name`: Tool name (may be empty on early updates)
/// - `param_delta`: Structured parameter update (if `stream_processed_params` is enabled)
/// - `raw_param_delta`: Raw JSON parameter text (if `stream_processed_params` is disabled)
#[cfg_attr(feature = "python_ffi", pyclass(get_all))]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterToolCallDelta {
    /// Index of this tool call (0-based)
    pub index: usize,
    /// Tool call identifier (CMD3+ only)
    pub id: String,
    /// Name of the tool being called
    pub name: String,
    /// Structured parameter delta (if enabled)
    pub param_delta: Option<FilterToolParameter>,
    /// Raw JSON parameter text
    pub raw_param_delta: String,
}

/// A parsed tool parameter update.
///
/// When `stream_processed_params` is enabled, parameters are parsed into
/// name-value pairs. The value is streamed incrementally as `value_delta`.
///
/// # Examples
///
/// ```rust
/// use cohere_melody::FilterToolParameter;
///
/// let param = FilterToolParameter {
///     name: "query".to_string(),
///     value_delta: "\"hello".to_string(),
/// };
/// assert_eq!(param.name, "query");
/// ```
#[cfg_attr(feature = "python_ffi", pyclass(get_all))]
#[derive(Debug, Clone, PartialEq)]
pub struct FilterToolParameter {
    /// Parameter name
    pub name: String,
    /// Incremental parameter value (may be partial JSON)
    pub value_delta: String,
}

/// A citation parsed from the model output with source attribution.
///
/// Citations indicate which parts of the generated text are grounded in specific
/// source documents or tool results. The citation includes character indices that
/// map to the position in the overall text output.
///
/// # Format Support
///
/// - **Legacy format**: `<co: 1,2>text</co: 1,2>` (single tool call, multiple results)
/// - **CMD3+ format**: `<co>text</co: 0:[1,2],1:[0]>` (multiple tool calls with result indices)
///
/// # Examples
///
/// ```rust
/// use cohere_melody::{FilterCitation, Source};
///
/// let citation = FilterCitation {
///     start_index: 6,
///     end_index: 11,
///     text: "world".to_string(),
///     sources: vec![Source {
///         tool_call_index: 0,
///         tool_result_indices: vec![0, 1],
///     }],
///     is_thinking: false,
/// };
/// assert_eq!(citation.text, "world");
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "python_ffi", pyclass(get_all))]
pub struct FilterCitation {
    /// Character index where the citation starts in the overall text output.
    /// For example, in "Hello world", if "world" is cited, `start_index` would be 6.
    pub start_index: usize,
    /// Character index where the citation ends (exclusive) in the overall text output.
    /// For example, in "Hello world", if "world" is cited, `end_index` would be 11.
    pub end_index: usize,
    /// The actual cited text content
    pub text: String,
    /// Source documents/results that ground this citation
    pub sources: Vec<Source>,
    /// True if this citation appears in a thinking/reasoning block
    pub is_thinking: bool,
}

/// Source attribution for a citation.
///
/// This structure identifies which tool call and which specific results from that
/// tool call are being cited. A single citation may reference multiple sources.
///
/// # Examples
///
/// ```rust
/// use cohere_melody::Source;
///
/// let source = Source {
///     tool_call_index: 0,
///     tool_result_indices: vec![0, 1, 2],
/// };
/// // This means the citation references results 0, 1, and 2 from tool call 0
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "python_ffi", pyclass(get_all))]
pub struct Source {
    /// Index of the tool call that produced these results
    pub tool_call_index: usize,
    /// Indices of specific results from this tool call
    pub tool_result_indices: Vec<usize>,
}

/// Parsing mode for the filter state machine.
///
/// The filter uses a state machine that transitions between different modes based on
/// special tokens encountered in the stream. Each mode determines how subsequent
/// tokens are processed.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "python_ffi", pyclass(eq, eq_int))]
pub enum FilterMode {
    /// Output all text without special processing
    PlainText,
    /// Discard all tokens (no output)
    Ignore,
    /// Parse tool calls from JSON-formatted action blocks
    ToolAction,
    /// Parse thinking/reasoning blocks (may be filtered based on config)
    ToolReason,
    /// Parse non-grounded answer text
    Answer,
    /// Parse grounded answer with citation extraction
    GroundedAnswer,
    /// Stop parsing and include the stop token in output
    InclusiveStop,
    /// Stop parsing and exclude the stop token from output
    ExclusiveStop,
    /// Parse search query content
    SearchQuery,
    /// Transition marker for next search query
    NextSearchQuery,
}
