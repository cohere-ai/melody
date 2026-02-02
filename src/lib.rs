#![warn(missing_docs)]
//! Melody Parsing - Rust port of the Cohere token stream parsing library
//!
//! This library provides functionality for parsing and filtering token streams from language models,
//! with support for citations, tool calls, and various output formats.
//!
//! # Overview
//!
//! Melody is a streaming parser for Cohere's language model outputs. It processes tokens as they're
//! generated and extracts structured information such as:
//! - Citations with source attribution
//! - Tool calls and their parameters
//! - Search queries
//! - Reasoning steps (thinking)
//! - Regular text content
//!
//! # Features
//!
//! - **Streaming Processing**: Parse tokens incrementally as they arrive
//! - **Citation Extraction**: Parse inline citations with source tracking
//! - **Tool Call Parsing**: Extract tool names, IDs, and parameters from structured outputs
//! - **Multiple Format Support**: Handles CMD3, CMD4, RAG, and multi-hop formats
//! - **FFI Support**: C and Python bindings for cross-language usage
//! - **Configurable Filtering**: Control what content is streamed vs. buffered
//!
//! # Quick Start
//!
//! ```rust
//! use cohere_melody::parsing::{FilterOptions, new_filter, Filter};
//!
//! // Create a filter with CMD3 configuration
//! let options = FilterOptions::new().cmd3();
//! let mut filter = new_filter(options);
//!
//! // Process tokens as they arrive
//! let outputs = filter.write_decoded("Hello world", Default::default());
//! for output in outputs {
//!     println!("Text: {}", output.text);
//! }
//!
//! // Flush any remaining partial outputs
//! let final_outputs = filter.flush_partials();
//! ```
//!
//! # Usage Patterns
//!
//! ## Basic Text Filtering
//!
//! ```rust
//! use cohere_melody::parsing::{FilterOptions, new_filter, Filter};
//!
//! let options = FilterOptions::new()
//!     .with_left_trimmed()  // Trim leading whitespace
//!     .with_right_trimmed(); // Trim trailing whitespace
//! let mut filter = new_filter(options);
//!
//! let outputs = filter.write_decoded("  Hello  ", Default::default());
//! assert_eq!(outputs[0].text, "Hello");
//! ```
//!
//! ## Citation Parsing (CMD3 Format)
//!
//! ```rust
//! use cohere_melody::parsing::{FilterOptions, new_filter, Filter};
//!
//! let options = FilterOptions::new().cmd3();
//! let mut filter = new_filter(options);
//!
//! // The filter will parse citations from the token stream
//! let outputs = filter.write_decoded("<START_RESPONSE>", Default::default());
//! // Continue feeding tokens...
//! ```
//!
//! ## Tool Call Extraction
//!
//! ```rust
//! use cohere_melody::parsing::{FilterOptions, new_filter, Filter};
//!
//! let options = FilterOptions::new()
//!     .cmd3()
//!     .stream_tool_actions();
//! let mut filter = new_filter(options);
//!
//! // Tool calls will be extracted and returned in FilterOutput.tool_call_delta
//! ```
//!
//! # Architecture
//!
//! The library uses a state machine approach to parse token streams:
//! - `FilterImpl`: Main state machine that processes tokens
//! - `FilterMode`: Different parsing modes (`PlainText`, `ToolAction`, `GroundedAnswer`, etc.)
//! - `FilterOptions`: Configuration for the filter behavior
//! - `FilterOutput`: Structured output containing parsed text, citations, and tool calls
//!
//! # Safety
//!
//! The core Rust library is safe. The FFI modules (`ffi` and `python_ffi`) contain
//! unsafe code for cross-language interoperability. See their respective module
//! documentation for safety requirements.

/// Error types for the Melody library.
pub mod errors;

/// Parsing module for token stream processing and filtering.
///
/// Contains the filter implementation, options, and types for processing
/// cohere model outputs with support for citations, tool calls, and structured content.
pub mod parsing;

/// Templating module for rendering prompts.
///
/// Provides functionality to render cohere Command 3 & Command 4 format prompts with support
/// for messages, tools, documents, and various configuration options.
pub mod templating;

// FFI bindings for calling from other languages (Go, Python, etc.)
#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "python_ffi")]
mod python_ffi;

#[cfg(feature = "tkzrs")]
/// Tokenizer FFI bindings for cross-language tokenization support.
///
/// This module provides C-compatible bindings for `HuggingFace` tokenizers,
/// primarily intended for use from Go but compatible with any language that
/// can call C functions.
///
/// # Features
///
/// - Load tokenizers from bytes or file
/// - Encode text to token IDs with various options
/// - Decode token IDs back to text
/// - Query vocabulary size
/// - Configurable truncation and special token handling
///
/// # Safety
///
/// All functions in this module are `unsafe extern "C"` and require careful
/// memory management. See the module documentation for details on ownership
/// and memory safety requirements.
pub mod tokenizers;

#[cfg(test)]
mod tests;
