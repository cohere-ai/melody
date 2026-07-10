//! Python bindings for the Melody parsing library.
//!
//! Provides `PyFilter` for parsing and `render_cmd3`/`render_cmd4`/`render_cmd5` for templating.

use crate::parsing::{AccumulatedToolCall, FilterAggregatedResult, SearchQueryDelta};
use crate::parsing::{Filter, FilterImpl, FilterOptions, new_filter};
use crate::templating::{
    RenderCmd3Options, RenderCmd4Options, RenderCmd5Options, render_cmd3 as rust_render_cmd3,
    render_cmd4 as rust_render_cmd4, render_cmd5 as rust_render_cmd5,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::depythonize;
use serde_json::Value;

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

    /// Configure for Command 5 format (cofl-tagged tool calls).
    fn cmd5(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().cmd5(),
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

    /// Disable tool call parsing by removing the action tokens.
    fn no_tools(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().no_tools(),
        }
    }

    /// Disable XML entity decoding for cofl parameter bodies.
    fn cofl_no_xml_text_decode(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().cofl_no_xml_text_decode(),
        }
    }

    /// Parse cofl tool parameters as nested `<cofl:value>` nodes.
    fn cofl_nested_xml(&self) -> Self {
        PyFilterOptions {
            inner: self.inner.clone().cofl_nested_xml(),
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
///     result = filter.write_decoded(token)
///     if result.content:
///         print(result.content)
/// result = filter.flush_partials()
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

    /// Create a filter for Command 5 format (cofl-tagged tool calls).
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Characters to buffer before emitting (default: 1)
    #[staticmethod]
    #[pyo3(signature = (chunk_size = None))]
    fn cmd5(chunk_size: Option<usize>) -> Self {
        let mut opts = FilterOptions::default().cmd5();
        if let Some(size) = chunk_size {
            opts = opts.with_chunk_size(size);
        }
        PyFilter {
            inner: new_filter(opts),
            config: "cmd5",
        }
    }

    /// Process a decoded token and return an aggregated result.
    fn write_decoded(&mut self, decoded_token: &str) -> FilterAggregatedResult {
        self.inner.write_decoded(decoded_token)
    }

    /// Flush any buffered partial outputs.
    fn flush_partials(&mut self) -> FilterAggregatedResult {
        self.inner.flush_partials()
    }

    /// Process a complete output token-by-token and return a single result
    /// with fully accumulated tool calls.
    #[allow(clippy::needless_pass_by_value)]
    fn process_full(&mut self, token_strings: Vec<String>) -> FilterAggregatedResult {
        self.inner.process_full(&token_strings)
    }

    /// Process a complete model output string in one call.
    ///
    /// The text is split at special token boundaries internally in Rust.
    fn process_full_text(&mut self, text: &str) -> FilterAggregatedResult {
        self.inner.process_full_text(text)
    }

    /// Classify decoded chunks by whether they emit content.
    #[allow(clippy::needless_pass_by_value)]
    fn classify_content_chunks(&mut self, token_strings: Vec<String>) -> Vec<bool> {
        self.inner.classify_content_chunks(&token_strings)
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

/// Render a Command 5 format prompt.
///
/// CMD5 accepts the same configuration as CMD4 (see `render_cmd4`); the
/// difference is the underlying Jinja template that is used.
///
/// # Arguments
///
/// * `config` - Dict with rendering options (same shape as `render_cmd4`).
///
/// # Returns
///
/// The rendered prompt string.
///
/// # Example
///
/// ```python
/// render_cmd5({"messages": [{"role": "user", "content": [{"type": "text", "text": "Hi"}]}]})
/// ```
#[pyfunction]
#[allow(clippy::needless_pass_by_value)] // PyO3's FromPyObject extracts owned values
fn render_cmd5(config: PyDictValue) -> PyResult<String> {
    let opts: RenderCmd5Options = serde_path_to_error::deserialize(&config.0)
        .map_err(|e| PyValueError::new_err(format!("Invalid config: {e}")))?;
    rust_render_cmd5(&opts).map_err(|e| PyValueError::new_err(format!("Render error: {e}")))
}

#[pymodule]
fn cohere_melody(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFilterOptions>()?;
    m.add_class::<PyFilter>()?;
    m.add_class::<AccumulatedToolCall>()?;
    m.add_class::<FilterAggregatedResult>()?;
    m.add_class::<SearchQueryDelta>()?;
    m.add_function(wrap_pyfunction!(render_cmd3, m)?)?;
    m.add_function(wrap_pyfunction!(render_cmd4, m)?)?;
    m.add_function(wrap_pyfunction!(render_cmd5, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_write_decoded() {
        let mut filter = PyFilter::cmd3(None);
        let result = filter.write_decoded("Hello");
        assert_eq!(result.content, Some("Hello".to_string()));
        assert!(result.reasoning.is_none());
    }

    #[test]
    fn test_filter_cmd5_cofl_tool_call() {
        let mut filter = PyFilter::cmd5(None);
        let text = concat!(
            "<|START_THINKING|>thinking<|END_THINKING|>",
            "<cofl:tool_calls><cofl:tool_call id=\"0\" name=\"search\">",
            "<cofl:tool_param name=\"q\" string=\"true\">hello</cofl:tool_param>",
            "</cofl:tool_call></cofl:tool_calls>"
        );
        let result = filter.process_full_text(text);
        assert_eq!(result.reasoning, Some("thinking".to_string()));
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "search");
        assert_eq!(result.tool_calls[0].arguments, r#"{"q": "hello"}"#);
    }

    #[test]
    fn test_filter_flush_partials() {
        let mut filter = PyFilter::cmd3(None);
        filter.write_decoded("Hello world");
        let result = filter.flush_partials();
        let text = result.content.unwrap_or_default();
        assert!(text.contains("Hello") || text.is_empty());
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
    fn test_render_cmd5_empty() {
        let json = r#"{"messages": []}"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let opts: RenderCmd5Options = serde_path_to_error::deserialize(&value).unwrap();
        let result = rust_render_cmd5(&opts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_cmd5_with_message() {
        let json = r#"{
            "messages": [{"role": "user", "content": [{"type": "text", "text": "Hello!"}]}]
        }"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let opts: RenderCmd5Options = serde_path_to_error::deserialize(&value).unwrap();
        let result = rust_render_cmd5(&opts).unwrap();
        assert!(result.contains("USER"));
        assert!(result.contains("Hello!"));
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
}
