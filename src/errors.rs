//! Melody Errors
//!
//! This module documents the errors that can occur in the Melody library.
//!

use thiserror::Error;

/// Errors that can occur in the Melody library
#[derive(Error, Debug)]
pub enum MelodyError {
    /// Unknown error
    #[error("unknown melody error")]
    Unknown,

    /// JSON serialization error
    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    /// Template parsing error
    #[error("Template parsing error: {0}")]
    TemplateParsing(#[from] liquid::Error),

    /// Template parsing error
    #[error("Template parsing error: {0}")]
    JinjaTemplateParsing(#[from] minijinja::Error),

    /// Validation error
    #[error("Template validation error: {0}")]
    TemplateValidation(String),
}
