//! Configuration options for creating filters
//!
//! This module provides the `FilterOptions` builder for configuring filter behavior.

use crate::parsing::filter::FilterImpl;
use crate::parsing::types::FilterMode;
use std::collections::HashMap;

/// Configuration builder for creating filters.
///
/// This struct uses the builder pattern to configure filter behavior before creating
/// a `FilterImpl` instance. It supports preset configurations for different Cohere
/// model output formats (Command 3, Command 4, etc.) as well as fine-grained control.
///
/// # Examples
///
/// ## Using presets
///
/// ```rust
/// use cohere_melody::parsing::FilterOptions;
/// use cohere_melody::parsing::new_filter;
///
/// // Use Command 3 preset configuration
/// let options = FilterOptions::new().cmd3();
/// let filter = new_filter(options);
/// ```
#[derive(Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct FilterOptions {
    pub(crate) left_trimmed: bool,
    pub(crate) right_trimmed: bool,
    pub(crate) inclusive_stops: Vec<String>,
    pub(crate) exclusive_stops: Vec<String>,
    pub(crate) chunk_size: usize,
    pub(crate) special_token_map: HashMap<String, FilterMode>,
    pub(crate) default_mode: FilterMode,
    pub(crate) stream_non_grounded_answer: bool,
    pub(crate) stream_tool_actions: bool,
    pub(crate) stream_processed_params: bool,
    pub(crate) has_tool_call_id: bool,
    pub(crate) cmd3_citations: bool,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self {
            left_trimmed: false,
            right_trimmed: false,
            inclusive_stops: Vec::new(),
            exclusive_stops: Vec::new(),
            chunk_size: 1,
            special_token_map: HashMap::new(),
            default_mode: FilterMode::PlainText,
            stream_non_grounded_answer: false,
            stream_tool_actions: false,
            stream_processed_params: false,
            has_tool_call_id: false,
            cmd3_citations: false,
        }
    }
}

impl FilterOptions {
    /// Creates a new `FilterOptions` with default settings.
    ///
    /// Default configuration:
    /// - No trimming
    /// - No stop sequences
    /// - Chunk size of 1
    /// - Plain text mode
    /// - No streaming of tool actions or parameters
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // CONFIGURATION FOR BINDINGS

    /// Configure for Cohere Command 3 model format.
    ///
    /// Command 3 is a structured output format that uses special tokens to delimit
    /// different sections of the response:
    /// - `<|START_RESPONSE|>`: Begin grounded answer
    /// - `<|END_RESPONSE|>`: End response
    /// - `<|START_THINKING|>`: Begin reasoning block
    /// - `<|END_THINKING|>`: End reasoning block
    /// - `<|START_ACTION|>`: Begin tool call
    /// - `<|END_ACTION|>`: End tool call
    ///
    /// This preset enables:
    /// - Grounded answer parsing with citations (Command 3 citation format)
    /// - Tool action streaming
    /// - Right trimming
    /// - Tool call ID support
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::{FilterOptions, new_filter};
    ///
    /// let options = FilterOptions::new().cmd3();
    /// let mut filter = new_filter(options);
    /// ```
    #[must_use]
    pub fn cmd3(mut self) -> Self {
        self.default_mode = FilterMode::GroundedAnswer;
        self.right_trimmed = true;
        self.has_tool_call_id = true;
        self.cmd3_citations = true;
        self.stream_tool_actions = true;
        self.special_token_map
            .insert("<|START_RESPONSE|>".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("<|END_RESPONSE|>".to_string(), FilterMode::Ignore);
        self.special_token_map
            .insert("<|START_THINKING|>".to_string(), FilterMode::ToolReason);
        self.special_token_map
            .insert("<|END_THINKING|>".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("<|START_ACTION|>".to_string(), FilterMode::ToolAction);
        self.special_token_map
            .insert("<|END_ACTION|>".to_string(), FilterMode::Ignore);
        self
    }

    /// Configure for Cohere Command 4 model format.
    ///
    /// Command 4 is similar to Command 3 but uses slightly different special tokens:
    /// - `<|START_TEXT|>`: Begin grounded answer (instead of `START_RESPONSE`)
    /// - `<|END_TEXT|>`: End text (instead of `END_RESPONSE`)
    ///
    /// All other special tokens and behavior are the same as Command 3.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::{FilterOptions, new_filter};
    ///
    /// let options = FilterOptions::new().cmd4();
    /// let mut filter = new_filter(options);
    /// ```
    #[must_use]
    pub fn cmd4(mut self) -> Self {
        self.default_mode = FilterMode::GroundedAnswer;
        self.right_trimmed = true;
        self.has_tool_call_id = true;
        self.cmd3_citations = true;
        self.stream_tool_actions = true;
        self.special_token_map
            .insert("<|START_TEXT|>".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("<|END_TEXT|>".to_string(), FilterMode::Ignore);
        self.special_token_map
            .insert("<|START_THINKING|>".to_string(), FilterMode::ToolReason);
        self.special_token_map
            .insert("<|END_THINKING|>".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("<|START_ACTION|>".to_string(), FilterMode::ToolAction);
        self.special_token_map
            .insert("<|END_ACTION|>".to_string(), FilterMode::Ignore);
        self
    }

    /// Add inclusive stop sequences.
    ///
    /// Inclusive stops will halt parsing when encountered, but the stop sequence
    /// itself will be included in the output.
    ///
    /// # Arguments
    ///
    /// * `stops` - Vector of stop sequences to recognize
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new()
    ///     .with_inclusive_stops(vec!["DONE".to_string()]);
    /// ```
    #[must_use]
    pub fn with_inclusive_stops(mut self, stops: Vec<String>) -> Self {
        self.inclusive_stops = stops;
        self
    }

    /// Add exclusive stop sequences.
    ///
    /// Exclusive stops will halt parsing when encountered, and the stop sequence
    /// will NOT be included in the output.
    ///
    /// # Arguments
    ///
    /// * `stops` - Vector of stop sequences to recognize
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new()
    ///     .with_exclusive_stops(vec!["</output>".to_string()]);
    /// ```
    #[must_use]
    pub fn with_exclusive_stops(mut self, stops: Vec<String>) -> Self {
        self.exclusive_stops = stops;
        self
    }

    // INTERNAL USE OPTIONS

    /// Enable left trimming of whitespace from outputs.
    ///
    /// When enabled, leading whitespace will be removed from text outputs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new().with_left_trimmed();
    /// ```
    #[must_use]
    pub fn with_left_trimmed(mut self) -> Self {
        self.left_trimmed = true;
        self
    }

    /// Enable right trimming of whitespace from outputs.
    ///
    /// When enabled, trailing whitespace will be removed from text outputs.
    /// This is commonly used in CMD3/CMD4 configurations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new().with_right_trimmed();
    /// ```
    #[must_use]
    pub fn with_right_trimmed(mut self) -> Self {
        self.right_trimmed = true;
        self
    }

    /// Set the chunk size for output batching.
    ///
    /// Determines how many characters to accumulate before emitting an output.
    /// A chunk size of 1 means immediate streaming of every character.
    ///
    /// # Arguments
    ///
    /// * `size` - Number of characters to buffer before emitting
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new().with_chunk_size(10);
    /// ```
    #[must_use]
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Configure for RAG (Retrieval Augmented Generation) format.
    ///
    /// This preset is for older RAG-style outputs that use text markers like
    /// "Grounded answer:" and "Answer:" to delimit different sections.
    ///
    /// Enables:
    /// - Right trimming
    /// - Recognition of "Grounded answer:" marker
    /// - Recognition of "Answer:" marker
    /// - Default mode: Ignore (content only appears after markers)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::{FilterOptions, new_filter};
    ///
    /// let options = FilterOptions::new().handle_rag();
    /// let mut filter = new_filter(options);
    /// ```
    #[must_use]
    pub fn handle_rag(mut self) -> Self {
        self.default_mode = FilterMode::Ignore;
        self.right_trimmed = true;
        self.special_token_map
            .insert("Grounded answer:".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("Answer:".to_string(), FilterMode::Answer);
        self
    }

    /// Configure for search query parsing format.
    ///
    /// This preset extracts search queries from model outputs. Search queries
    /// appear after "Search:" markers and can be separated by "|||" or newlines
    /// for multiple queries.
    ///
    /// Enables:
    /// - Right trimming
    /// - Recognition of "Search:" marker
    /// - Multi-query support (separated by "|||" or newline)
    /// - Default mode: Ignore (only emit search queries)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::{FilterOptions, new_filter};
    ///
    /// let options = FilterOptions::new().handle_search_query();
    /// let mut filter = new_filter(options);
    /// ```
    #[must_use]
    pub fn handle_search_query(mut self) -> Self {
        self.default_mode = FilterMode::Ignore;
        self.right_trimmed = true;
        self.special_token_map
            .insert("Search:".to_string(), FilterMode::SearchQuery);
        self.special_token_map
            .insert("|||".to_string(), FilterMode::NextSearchQuery);
        self.special_token_map
            .insert("\n".to_string(), FilterMode::NextSearchQuery);
        self
    }

    /// Configure for multi-hop reasoning format.
    ///
    /// Multi-hop is an older format that uses text markers to delimit different
    /// reasoning steps and actions. It includes planning, reflection, and tool
    /// execution phases.
    ///
    /// Enables:
    /// - Right trimming
    /// - Recognition of "Plan:" and "Reflection:" (reasoning blocks)
    /// - Recognition of "Action:" (tool calls)
    /// - Recognition of "Grounded answer:" and "Answer:" (final output)
    /// - Filtering of document listing sections
    /// - Default mode: Ignore
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::{FilterOptions, new_filter};
    ///
    /// let options = FilterOptions::new().handle_multi_hop();
    /// let mut filter = new_filter(options);
    /// ```
    #[must_use]
    pub fn handle_multi_hop(mut self) -> Self {
        self.default_mode = FilterMode::Ignore;
        self.right_trimmed = true;
        self.special_token_map
            .insert("Grounded answer:".to_string(), FilterMode::GroundedAnswer);
        self.special_token_map
            .insert("Answer:".to_string(), FilterMode::Answer);
        self.special_token_map
            .insert("Plan:".to_string(), FilterMode::ToolReason);
        self.special_token_map
            .insert("Reflection:".to_string(), FilterMode::ToolReason);
        self.special_token_map
            .insert("Action:".to_string(), FilterMode::ToolAction);
        self.special_token_map
            .insert("Relevant Documents:".to_string(), FilterMode::Ignore);
        self.special_token_map
            .insert("Cited Documents:".to_string(), FilterMode::Ignore);
        self
    }

    /// Enable streaming of non-grounded answer content.
    ///
    /// When enabled, content in "Answer:" sections (non-grounded answers without
    /// citations) will be streamed in addition to grounded answers.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new()
    ///     .handle_rag()
    ///     .stream_non_grounded_answer();
    /// ```
    #[must_use]
    pub fn stream_non_grounded_answer(mut self) -> Self {
        self.stream_non_grounded_answer = true;
        self
    }

    /// Enable streaming of tool action content.
    ///
    /// When enabled, tool calls will be parsed and streamed as they're generated.
    /// Tool calls appear as `FilterOutput.tool_call_delta` incremental updates.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new()
    ///     .cmd3()
    ///     .stream_tool_actions();
    /// ```
    #[must_use]
    pub fn stream_tool_actions(mut self) -> Self {
        self.stream_tool_actions = true;
        self
    }

    /// Enable streaming of processed (parsed) tool parameters.
    ///
    /// When enabled, tool parameters will be parsed into name-value pairs and
    /// streamed as `FilterToolParameter` objects. When disabled, raw JSON
    /// parameter text is returned instead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new()
    ///     .cmd3()
    ///     .stream_processed_params();
    /// ```
    #[must_use]
    pub fn stream_processed_params(mut self) -> Self {
        self.stream_processed_params = true;
        self
    }

    /// Remove a special token from the token map.
    ///
    /// Removes a previously configured special token, preventing it from
    /// triggering mode transitions.
    ///
    /// # Arguments
    ///
    /// * `token` - The special token to remove
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cohere_melody::parsing::FilterOptions;
    ///
    /// let options = FilterOptions::new()
    ///     .cmd3()
    ///     .remove_token("<|START_THINKING|>");
    /// ```
    #[must_use]
    pub fn remove_token(mut self, token: &str) -> Self {
        self.special_token_map.remove(token);
        self
    }
}

/// Creates a new filter with the specified options.
///
/// This is a convenience function that creates a `FilterImpl` and applies
/// the given configuration.
///
/// # Arguments
///
/// * `options` - Configuration options for the filter
///
/// # Returns
///
/// A configured `FilterImpl` ready to process tokens
///
/// # Examples
///
/// ```rust
/// use cohere_melody::parsing::{FilterOptions, new_filter, Filter};
///
/// let options = FilterOptions::new().cmd3();
/// let mut filter = new_filter(options);
///
/// let outputs = filter.write_decoded("Hello", Default::default());
/// ```
#[must_use]
pub fn new_filter(options: FilterOptions) -> FilterImpl {
    let filter = FilterImpl::new();
    filter.apply_options(options)
}
