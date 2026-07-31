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
//! Built-in templates are selected via `template_id` using:
//! - `{name}@{revision}` — exact pin (e.g. `cmd4-reasoning@1`)
//! - `{name}` — latest revision (e.g. `cmd4-reasoning`)
//!
//! Immutable archive files live at
//! `gen/templates/archive/{name}/{name}@{revision}.jinja`
//! and are embedded directly into Melody.

mod lib;

/// Type definitions for templating structures like messages, roles, and content.
pub mod types;

mod util;

#[path = "../../gen/template_registry.rs"]
mod template_registry;

pub use lib::*;
pub use template_registry::{ResolvedTemplate, TemplateMeta};
pub use types::*;
pub use util::PromptRenderIds;

use crate::errors::MelodyError;

/// Resolve a template id to a built-in template.
///
/// Accepted forms: `{name}@{revision}` and `{name}`.
///
/// # Errors
///
/// Returns [`MelodyError::TemplateValidation`] when the id is unknown.
pub fn resolve_template_id(id: &str) -> Result<&'static ResolvedTemplate, MelodyError> {
    template_registry::resolve_template_id(id).map_err(MelodyError::TemplateValidation)
}

/// List registered templates.
pub fn list_templates() -> Vec<TemplateMeta> {
    template_registry::list_templates()
}
