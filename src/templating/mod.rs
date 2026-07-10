//! Templating module for rendering CMD3, CMD4, and CMD5 prompts.
//!
//! This module provides functionality to render prompts with support for
//! messages, tools, documents, and various configuration options.
//!
//! CMD5 reuses the CMD4 option schema (`RenderCmd5Options` is a type alias
//! for `RenderCmd4Options`); the [`render_cmd5`] entry point differs only in
//! which jinja template is selected.

mod lib;

/// Type definitions for templating structures like messages, roles, and content.
pub mod types;

mod util;

pub use lib::*;
pub use types::*;
pub use util::PromptRenderIds;
