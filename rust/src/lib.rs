//! Melody Parsing - Rust port of the Cohere token stream parsing library
//!
//! This library provides functionality for parsing and filtering token streams from language models,
//! with support for citations, tool calls, and various output formats.

pub mod filter;
pub mod options;
pub mod templating;
pub mod types;

mod action_filter;
mod citations_filter;
mod param_filter;

#[cfg(test)]
mod tests;

pub use filter::{Filter, FilterImpl};
pub use options::{FilterOptions, new_filter};
pub use types::{
    FilterCitation, FilterOutput, FilterSearchQueryDelta, FilterToolCallDelta, FilterToolParameter,
    Source, TokenIDsWithLogProb,
};
