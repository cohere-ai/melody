//! Templating module for rendering CMD3, CMD4, and CMD5 prompts.
//!
//! This module provides functionality to render prompts with support for
//! messages, tools, documents, and various configuration options.
//!
//! CMD5 reuses the CMD4 option schema (`RenderCmd5Options` is a type alias
//! for `RenderCmd4Options`); the [`render_cmd5`] entry point differs only in
//! which jinja template is selected.
//!
//! # Template IDs
//!
//! Built-in templates can be selected via `template_id` using `{name}` (current
//! revision) or `{name}@{revision}` for any revision still present in
//! `gen/templates/archive/` (e.g. `cmd4`, `cmd4@1`). Only the current revision is
//! built from sources in `template_registry.yaml`; older revisions are frozen
//! raw archive files.

mod lib;

/// Type definitions for templating structures like messages, roles, and content.
pub mod types;

mod util;

#[path = "../../gen/embedded_templates.rs"]
mod embedded_templates;

pub use lib::*;
pub use types::*;
pub use util::PromptRenderIds;
