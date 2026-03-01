//! Python bindings for the Melody parsing library.
//!
//! Provides `PyFilter` for parsing and `render_cmd3`/`render_cmd4` for templating.

use crate::parsing::types::{FilterCitation, FilterOutput, FilterToolCallDelta, TokenIDsWithLogProb};
use crate::parsing::{Filter, FilterImpl, FilterOptions, new_filter};
use crate::templating::{
    RenderCmd3Options, RenderCmd4Options, render_cmd3 as rust_render_cmd3,
    render_cmd4 as rust_render_cmd4,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::depythonize;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Aggregated output types
// ---------------------------------------------------------------------------

/// Aggregated result of a streaming `write_decoded` call.
/// Pre-separates content, reasoning, and tool call deltas.
#[pyclass(get_all)]
#[derive(Debug, Clone, Default)]
struct AggregatedStreamResult {
    /// Non-reasoning text content (`None` if no text was produced).
    content: Option<String>,
    /// Reasoning/thinking text content (`None` if no reasoning was produced).
    reasoning: Option<String>,
    /// Tool call deltas produced in this chunk.
    tool_call_deltas: Vec<AggregatedToolCallDelta>,
    /// Citations produced in this chunk.
    citations: Vec<FilterCitation>,
}

/// A tool call delta with fields structured for the `OpenAI` API format.
#[pyclass(get_all)]
#[derive(Debug, Clone, Default)]
struct AggregatedToolCallDelta {
    /// Tool call index (0-based).
    index: usize,
    /// Tool call identifier.
    id: String,
    /// Tool name.
    name: String,
    /// Raw JSON parameter text delta.
    arguments: String,
}

/// A fully accumulated tool call (for unary/non-streaming responses).
#[pyclass(get_all)]
#[derive(Debug, Clone, Default)]
struct AccumulatedToolCall {
    /// Tool call index (0-based).
    index: usize,
    /// Tool call identifier.
    id: String,
    /// Tool name.
    name: String,
    /// Complete JSON arguments string.
    arguments: String,
}

/// Aggregated result of a full (unary) parse of model output.
#[pyclass(get_all)]
#[derive(Debug, Clone, Default)]
struct AggregatedFullResult {
    /// Non-reasoning text content.
    content: Option<String>,
    /// Reasoning/thinking text content.
    reasoning: Option<String>,
    /// Fully accumulated tool calls.
    tool_calls: Vec<AccumulatedToolCall>,
    /// All citations.
    citations: Vec<FilterCitation>,
}

// ---------------------------------------------------------------------------
// Aggregation logic
// ---------------------------------------------------------------------------

fn push_text(target: &mut Option<String>, text: &mut String) {
    match target {
        Some(s) => s.push_str(text),
        None => {
            *target = Some(std::mem::take(text));
        }
    }
}

fn push_tool_call_delta(
    deltas: &mut Vec<AggregatedToolCallDelta>,
    tc: FilterToolCallDelta,
) {
    deltas.push(AggregatedToolCallDelta {
        index: tc.index,
        id: tc.id,
        name: tc.name,
        arguments: tc.raw_param_delta,
    });
}

fn aggregate_stream(outputs: Vec<FilterOutput>) -> AggregatedStreamResult {
    let mut content: Option<String> = None;
    let mut reasoning: Option<String> = None;
    let mut tool_call_deltas = Vec::new();
    let mut citations = Vec::new();

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
            push_tool_call_delta(&mut tool_call_deltas, tc);
        }
    }

    AggregatedStreamResult {
        content,
        reasoning,
        tool_call_deltas,
        citations,
    }
}

fn aggregate_full(all_outputs: Vec<FilterOutput>) -> AggregatedFullResult {
    let mut content: Option<String> = None;
    let mut reasoning: Option<String> = None;
    let mut tool_calls: Vec<AccumulatedToolCall> = Vec::new();
    let mut citations = Vec::new();

    for mut o in all_outputs {
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
            if !tc.id.is_empty() {
                tool_calls.push(AccumulatedToolCall {
                    index: tc.index,
                    id: tc.id,
                    name: String::new(),
                    arguments: String::new(),
                });
            }
            if !tc.name.is_empty()
                && let Some(call) = tool_calls.get_mut(tc.index)
            {
                call.name = tc.name;
            }
            if let Some(call) = tool_calls.get_mut(tc.index) {
                call.arguments.push_str(&tc.raw_param_delta);
            }
        }
    }

    AggregatedFullResult {
        content,
        reasoning,
        tool_calls,
        citations,
    }
}

// ---------------------------------------------------------------------------
// PyO3 bindings
// ---------------------------------------------------------------------------

/// A Python dict extracted as a JSON value.
struct PyDictValue(Value);

impl<'a, 'py> FromPyObject<'a, 'py> for PyDictValue {
    type Error = PyErr;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let dict: Borrowed<'a, 'py, PyDict> = ob.cast()?;
        let value: Value = depythonize(&dict)
            .map_err(|e| PyValueError::new_err(format!("Invalid config: {e}")))?;
        Ok(PyDictValue(value))
    }
}

/// Configuration builder for creating filters.
///
/// Use the builder pattern to configure filter behavior.
///
/// # Example
///
/// ```python
/// opts = PyFilterOptions().cmd3().with_chunk_size(10)
/// filter = PyFilter(opts)
/// ```
#[pyclass]
#[derive(Clone)]
struct PyFilterOptions {
    inner: FilterOptions,
}

#[pymethods]
impl PyFilterOptions {
    /// Create new filter options with default settings.
    #[new]
    fn new() -> Self {
        PyFilterOptions {
            inner: FilterOptions::default(),
        }
    }

    /// Configure for Command 3 format.
    fn cmd3(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().cmd3(),
        }
    }

    /// Configure for Command 4 format.
    fn cmd4(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().cmd4(),
        }
    }

    /// Configure for RAG format.
    fn rag(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().handle_rag(),
        }
    }

    /// Configure for multi-hop reasoning format.
    fn multi_hop(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().handle_multi_hop(),
        }
    }

    /// Configure for search query extraction.
    fn search_query(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().handle_search_query(),
        }
    }

    /// Set the chunk size for output batching.
    fn with_chunk_size(&self, size: usize) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().with_chunk_size(size),
        }
    }

    /// Enable left trimming of whitespace.
    fn with_left_trimmed(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().with_left_trimmed(),
        }
    }

    /// Enable right trimming of whitespace.
    fn with_right_trimmed(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().with_right_trimmed(),
        }
    }

    /// Add inclusive stop sequences.
    fn with_inclusive_stops(&self, stops: Vec<String>) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().with_inclusive_stops(stops),
        }
    }

    /// Add exclusive stop sequences.
    fn with_exclusive_stops(&self, stops: Vec<String>) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().with_exclusive_stops(stops),
        }
    }

    /// Enable streaming of tool actions.
    fn stream_tool_actions(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().stream_tool_actions(),
        }
    }

    /// Enable streaming of processed tool parameters.
    fn stream_processed_params(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().stream_processed_params(),
        }
    }

    /// Enable streaming of non-grounded answers.
    fn stream_non_grounded_answer(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().stream_non_grounded_answer(),
        }
    }

    /// Remove a special token from the token map.
    fn remove_token(&self, token: &str) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().remove_token(token),
        }
    }

    #[allow(clippy::unused_self)]
    fn __repr__(&self) -> &'static str {
        "PyFilterOptions(...)"
    }
}

/// Streaming filter for parsing Cohere model outputs.
///
/// Create filters using `PyFilterOptions` or factory methods.
///
/// # Example with options
///
/// ```python
/// opts = PyFilterOptions().cmd3().with_chunk_size(10)
/// filter = PyFilter(opts)
/// ```
///
/// # Example with factory methods
///
/// ```python
/// filter = PyFilter.cmd3()
/// for token in tokens:
///     for output in filter.write_decoded(token):
///         print(output.text)
/// for output in filter.flush_partials():
///     print(output.text)
/// ```
#[pyclass]
struct PyFilter {
    inner: FilterImpl,
    config: &'static str,
}

#[pymethods]
impl PyFilter {
    /// Create a filter from `PyFilterOptions`.
    #[new]
    fn new(options: &PyFilterOptions) -> Self {
        PyFilter {
            inner: new_filter(options.inner.clone()),
            config: "custom",
        }
    }

    /// Create a filter for Command 3 format.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Characters to buffer before emitting (default: 1)
    #[staticmethod]
    #[pyo3(signature = (chunk_size = None))]
    fn cmd3(chunk_size: Option<usize>) -> Self {
        let mut opts = FilterOptions::default().cmd3();
        if let Some(size) = chunk_size {
            opts = opts.with_chunk_size(size);
        }
        PyFilter {
            inner: new_filter(opts),
            config: "cmd3",
        }
    }

    /// Create a filter for Command 4 format.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Characters to buffer before emitting (default: 1)
    #[staticmethod]
    #[pyo3(signature = (chunk_size = None))]
    fn cmd4(chunk_size: Option<usize>) -> Self {
        let mut opts = FilterOptions::default().cmd4();
        if let Some(size) = chunk_size {
            opts = opts.with_chunk_size(size);
        }
        PyFilter {
            inner: new_filter(opts),
            config: "cmd4",
        }
    }

    /// Process a decoded token and return any completed outputs.
    ///
    /// # Arguments
    ///
    /// * `decoded_token` - The decoded text for this token
    ///
    /// # Returns
    ///
    /// List of `FilterOutput` objects (may be empty if content is buffered)
    fn write_decoded(&mut self, decoded_token: &str) -> Vec<FilterOutput> {
        self.inner
            .write_decoded(decoded_token, TokenIDsWithLogProb::new())
    }

    /// Flush any buffered partial outputs.
    ///
    /// Call this at the end of generation to get remaining content.
    ///
    /// # Returns
    ///
    /// List of remaining `FilterOutput` objects
    fn flush_partials(&mut self) -> Vec<FilterOutput> {
        self.inner.flush_partials()
    }

    /// Process a decoded token and return an aggregated streaming result.
    ///
    /// Replaces the pattern of calling `write_decoded()` then looping
    /// over `FilterOutput`s in Python to separate content, reasoning, and tool calls.
    fn write_decoded_aggregated(&mut self, decoded_token: &str) -> AggregatedStreamResult {
        let outputs = self
            .inner
            .write_decoded(decoded_token, TokenIDsWithLogProb::new());
        aggregate_stream(outputs)
    }

    /// Process a complete model output token-by-token.
    ///
    /// The caller provides decoded token strings (one per token, already
    /// buffered for complete UTF-8). This method feeds each through the
    /// filter, flushes partials, and returns a single aggregated result
    /// with fully accumulated tool calls.
    #[allow(clippy::needless_pass_by_value)] // PyO3 requires owned Vec for Python interop
    fn process_full(&mut self, token_strings: Vec<String>) -> AggregatedFullResult {
        let mut all_outputs = Vec::new();
        for token_str in &token_strings {
            all_outputs.extend(
                self.inner
                    .write_decoded(token_str, TokenIDsWithLogProb::new()),
            );
        }
        all_outputs.extend(self.inner.flush_partials());
        aggregate_full(all_outputs)
    }

    /// Flush and aggregate any remaining buffered outputs.
    fn flush_aggregated(&mut self) -> AggregatedStreamResult {
        let outputs = self.inner.flush_partials();
        aggregate_stream(outputs)
    }

    fn __repr__(&self) -> String {
        format!("PyFilter(\"{}\")", self.config)
    }
}

/// Render a Command 3 format prompt.
///
/// # Arguments
///
/// * `config` - Dict with rendering options:
///   - `messages` (required): List of message dicts to include in the prompt.
///   - `dev_instruction` (optional): Developer instruction to include.
///   - `documents` (optional): Documents to include for grounding.
///   - `available_tools` (optional): Tools available to the model.
///   - `safety_mode` (optional): Safety mode configuration.
///   - `citation_quality` (optional): Citation quality setting (default: "on").
///   - `reasoning_type` (optional): Reasoning/thinking mode configuration.
///   - `skip_preamble` (optional): Whether to skip the preamble section (default: false).
///   - `response_prefix` (optional): Prefix for the response.
///   - `json_schema` (optional): JSON schema for structured output.
///   - `json_mode` (optional): Whether to enable JSON mode (default: false).
///   - `additional_template_fields` (optional): Additional fields to substitute in the template.
///   - `escaped_special_tokens` (optional): Special tokens to escape in the output.
///
/// # Returns
///
/// The rendered prompt string.
///
/// # Example
///
/// ```python
/// render_cmd3({"messages": [{"role": "user", "content": [{"type": "text", "text": "Hi"}]}]})
/// ```
#[pyfunction]
#[allow(clippy::needless_pass_by_value)] // PyO3's FromPyObject extracts owned values
fn render_cmd3(config: PyDictValue) -> PyResult<String> {
    let opts: RenderCmd3Options = serde_path_to_error::deserialize(&config.0)
        .map_err(|e| PyValueError::new_err(format!("Invalid config: {e}")))?;
    rust_render_cmd3(&opts).map_err(|e| PyValueError::new_err(format!("Render error: {e}")))
}

/// Render a Command 4 format prompt.
///
/// # Arguments
///
/// * `config` - Dict with rendering options:
///   - `messages` (required): List of message dicts to include in the prompt.
///   - `dev_instruction` (optional): Developer instruction to include.
///   - `platform_instruction` (optional): Platform instruction override.
///   - `documents` (optional): Documents to include for grounding.
///   - `available_tools` (optional): Tools available to the model.
///   - `grounding` (optional): Grounding configuration (default: "enabled").
///   - `response_prefix` (optional): Prefix for the response.
///   - `json_schema` (optional): JSON schema for structured output.
///   - `json_mode` (optional): Whether to enable JSON mode (default: false).
///   - `additional_template_fields` (optional): Additional fields to substitute in the template.
///   - `escaped_special_tokens` (optional): Special tokens to escape in the output.
///
/// # Returns
///
/// The rendered prompt string.
///
/// # Example
///
/// ```python
/// render_cmd4({"messages": [{"role": "user", "content": [{"type": "text", "text": "Hi"}]}]})
/// ```
#[pyfunction]
#[allow(clippy::needless_pass_by_value)] // PyO3's FromPyObject extracts owned values
fn render_cmd4(config: PyDictValue) -> PyResult<String> {
    let opts: RenderCmd4Options = serde_path_to_error::deserialize(&config.0)
        .map_err(|e| PyValueError::new_err(format!("Invalid config: {e}")))?;
    rust_render_cmd4(&opts).map_err(|e| PyValueError::new_err(format!("Render error: {e}")))
}

#[pymodule]
fn cohere_melody(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFilterOptions>()?;
    m.add_class::<PyFilter>()?;
    m.add_class::<AggregatedStreamResult>()?;
    m.add_class::<AggregatedToolCallDelta>()?;
    m.add_class::<AccumulatedToolCall>()?;
    m.add_class::<AggregatedFullResult>()?;
    m.add_function(wrap_pyfunction!(render_cmd3, m)?)?;
    m.add_function(wrap_pyfunction!(render_cmd4, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_write_decoded() {
        let mut filter = PyFilter::cmd3(None);
        let outputs = filter.write_decoded("Hello");
        assert!(outputs.is_empty() || !outputs[0].text.is_empty());
    }

    #[test]
    fn test_filter_flush_partials() {
        let mut filter = PyFilter::cmd3(None);
        filter.write_decoded("Hello world");
        let outputs = filter.flush_partials();
        let text: String = outputs.iter().map(|o| o.text.as_str()).collect();
        assert!(text.contains("Hello"));
    }

    #[test]
    fn test_render_cmd3_empty() {
        let json = r#"{"messages": []}"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let opts: RenderCmd3Options = serde_path_to_error::deserialize(&value).unwrap();
        let result = rust_render_cmd3(&opts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_cmd4_empty() {
        let json = r#"{"messages": []}"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let opts: RenderCmd4Options = serde_path_to_error::deserialize(&value).unwrap();
        let result = rust_render_cmd4(&opts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_cmd3_with_message() {
        let json = r#"{
            "messages": [{"role": "user", "content": [{"type": "text", "text": "Hello!"}]}]
        }"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let opts: RenderCmd3Options = serde_path_to_error::deserialize(&value).unwrap();
        let result = rust_render_cmd3(&opts).unwrap();
        assert!(result.contains("USER"));
        assert!(result.contains("Hello!"));
    }

    #[test]
    fn test_render_cmd4_with_message() {
        let json = r#"{
            "messages": [{"role": "user", "content": [{"type": "text", "text": "Hello!"}]}]
        }"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let opts: RenderCmd4Options = serde_path_to_error::deserialize(&value).unwrap();
        let result = rust_render_cmd4(&opts).unwrap();
        assert!(result.contains("USER"));
        assert!(result.contains("Hello!"));
    }

    #[test]
    fn test_render_cmd3_with_tools() {
        let json = r#"{
            "messages": [],
            "available_tools": [{
                "name": "search",
                "description": "Search the web",
                "parameters": {"type": "object"}
            }]
        }"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let opts: RenderCmd3Options = serde_path_to_error::deserialize(&value).unwrap();
        let result = rust_render_cmd3(&opts).unwrap();
        assert!(result.contains("search"));
    }

    // -- aggregation tests --

    #[test]
    fn test_aggregate_stream_empty() {
        let result = aggregate_stream(vec![]);
        assert!(result.content.is_none());
        assert!(result.reasoning.is_none());
        assert!(result.tool_call_deltas.is_empty());
    }

    #[test]
    fn test_aggregate_stream_content_only() {
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
        let result = aggregate_stream(outputs);
        assert_eq!(result.content, Some("hello world".into()));
        assert!(result.reasoning.is_none());
        assert!(result.tool_call_deltas.is_empty());
    }

    #[test]
    fn test_aggregate_stream_reasoning_only() {
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
        let result = aggregate_stream(outputs);
        assert!(result.content.is_none());
        assert_eq!(result.reasoning, Some("step 1 step 2".into()));
    }

    #[test]
    fn test_aggregate_stream_mixed() {
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

        let result = aggregate_stream(outputs);
        assert_eq!(result.reasoning, Some("thinking...".into()));
        assert_eq!(result.content, Some("hello".into()));
        assert_eq!(result.tool_call_deltas.len(), 1);
        assert_eq!(result.tool_call_deltas[0].id, "call_0");
        assert_eq!(result.tool_call_deltas[0].arguments, r#"{"q":"test"}"#);
    }

    #[test]
    fn test_aggregate_stream_with_citations() {
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
        let result = aggregate_stream(outputs);
        assert_eq!(result.citations.len(), 1);
        assert_eq!(result.citations[0].start_index, 0);
    }

    #[test]
    fn test_aggregate_stream_no_text_fields() {
        let outputs = vec![FilterOutput {
            text: String::new(),
            ..Default::default()
        }];
        let result = aggregate_stream(outputs);
        assert!(result.content.is_none());
        assert!(result.reasoning.is_none());
    }

    #[test]
    fn test_aggregate_full_empty() {
        let result = aggregate_full(vec![]);
        assert!(result.content.is_none());
        assert!(result.reasoning.is_none());
        assert!(result.tool_calls.is_empty());
        assert!(result.citations.is_empty());
    }

    #[test]
    fn test_aggregate_full_tool_calls() {
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

        let result = aggregate_full(outputs);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "0");
        assert_eq!(result.tool_calls[0].name, "search");
        assert_eq!(result.tool_calls[0].arguments, r#"{"q": "hello"}"#);
        assert_eq!(result.content, Some("Response text".into()));
    }

    #[test]
    fn test_aggregate_full_multiple_tool_calls() {
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

        let result = aggregate_full(outputs);
        assert_eq!(result.tool_calls.len(), 2);
        assert_eq!(result.tool_calls[0].id, "call_0");
        assert_eq!(result.tool_calls[0].name, "search");
        assert_eq!(result.tool_calls[0].arguments, r#"{"q":"a"}"#);
        assert_eq!(result.tool_calls[1].id, "call_1");
        assert_eq!(result.tool_calls[1].name, "read");
        assert_eq!(result.tool_calls[1].arguments, r#"{"file":"b"}"#);
    }

    #[test]
    fn test_aggregate_full_reasoning_and_content() {
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
        let result = aggregate_full(outputs);
        assert_eq!(result.reasoning, Some("thinking".into()));
        assert_eq!(result.content, Some("answer".into()));
        assert!(result.tool_calls.is_empty());
    }
}
