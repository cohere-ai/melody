//! Templating module for rendering CMD3 and CMD4 prompts.
//!
//! This module provides functionality to render prompts with support for
//! messages, tools, documents, and various configuration options.

mod lib;

/// Type definitions for templating structures like messages, roles, and content.
pub mod types;

mod util;

pub use lib::*;
pub use types::*;
