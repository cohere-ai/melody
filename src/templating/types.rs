//! Type definitions for templating structures.
//!
//! This module contains types for representing messages, roles, content,
//! and various configuration options used in prompt rendering.

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::parsing::types::FilterCitation;

/// Role of a message in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Role {
    /// Unknown or unspecified role.
    Unknown,
    /// System message role.
    System,
    /// User message role.
    User,
    /// Chatbot/assistant message role.
    Chatbot,
    /// Tool response message role.
    Tool,
}

impl TryFrom<String> for Role {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "unknown" => Ok(Role::Unknown),
            "system" => Ok(Role::System),
            "user" => Ok(Role::User),
            "chatbot" => Ok(Role::Chatbot),
            "tool" => Ok(Role::Tool),
            other => Err(format!(
                "invalid Role '{other}', expected one of: unknown, system, user, chatbot, tool"
            )),
        }
    }
}

impl Role {
    /// Returns the string representation of the role.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Unknown => "UNKNOWN",
            Role::System => "SYSTEM",
            Role::User => "USER",
            Role::Chatbot => "CHATBOT",
            Role::Tool => "TOOL",
        }
    }
}

/// Type of content within a message.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum ContentType {
    /// Unknown or unspecified content type.
    Unknown,
    /// Plain text content.
    Text,
    /// Thinking/reasoning content.
    Thinking,
    /// Image content.
    Image,
    /// Document content.
    Document,
}

impl TryFrom<String> for ContentType {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "unknown" => Ok(ContentType::Unknown),
            "text" => Ok(ContentType::Text),
            "thinking" => Ok(ContentType::Thinking),
            "image" => Ok(ContentType::Image),
            "document" => Ok(ContentType::Document),
            other => Err(format!(
                "invalid ContentType '{other}', expected one of: unknown, text, thinking, image, document"
            )),
        }
    }
}

/// Citation quality setting for grounded responses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum CitationQuality {
    /// Unknown or unspecified citation quality.
    Unknown,
    /// Citations disabled.
    Off,
    /// Citations enabled.
    On,
}

impl TryFrom<String> for CitationQuality {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "unknown" => Ok(CitationQuality::Unknown),
            "off" => Ok(CitationQuality::Off),
            "on" => Ok(CitationQuality::On),
            other => Err(format!(
                "invalid CitationQuality '{other}', expected one of: unknown, off, on"
            )),
        }
    }
}

impl CitationQuality {
    /// Returns the string representation of the citation quality.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            CitationQuality::Unknown => "UNKNOWN",
            CitationQuality::Off => "OFF",
            CitationQuality::On => "ON",
        }
    }
}

/// Grounding configuration for document-based responses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Grounding {
    /// Unknown or unspecified grounding state.
    Unknown,
    /// Grounding enabled.
    Enabled,
    /// Grounding disabled.
    Disabled,
}

impl TryFrom<String> for Grounding {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "unknown" => Ok(Grounding::Unknown),
            "enabled" => Ok(Grounding::Enabled),
            "disabled" => Ok(Grounding::Disabled),
            other => Err(format!(
                "invalid Grounding '{other}', expected one of: unknown, enabled, disabled"
            )),
        }
    }
}

impl Grounding {
    /// Returns the string representation of the grounding setting.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Grounding::Unknown => "UNKNOWN",
            Grounding::Enabled => "ENABLED",
            Grounding::Disabled => "DISABLED",
        }
    }
}

/// Safety mode configuration for content filtering.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum SafetyMode {
    /// Unknown or unspecified safety mode.
    Unknown,
    /// No safety filtering.
    None,
    /// Strict safety filtering.
    Strict,
    /// Contextual safety filtering.
    Contextual,
}

impl TryFrom<String> for SafetyMode {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "unknown" => Ok(SafetyMode::Unknown),
            "none" => Ok(SafetyMode::None),
            "strict" => Ok(SafetyMode::Strict),
            "contextual" => Ok(SafetyMode::Contextual),
            other => Err(format!(
                "invalid SafetyMode '{other}', expected one of: unknown, none, strict, contextual"
            )),
        }
    }
}

impl SafetyMode {
    /// Returns the string representation of the safety mode.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyMode::Unknown => "UNKNOWN",
            SafetyMode::None => "NONE",
            SafetyMode::Strict => "STRICT",
            SafetyMode::Contextual => "CONTEXTUAL",
        }
    }
}

/// Reasoning/thinking mode configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum ReasoningType {
    /// Unknown or unspecified reasoning type.
    Unknown,
    /// Reasoning/thinking enabled.
    Enabled,
    /// Reasoning/thinking disabled.
    Disabled,
}

impl TryFrom<String> for ReasoningType {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "unknown" => Ok(ReasoningType::Unknown),
            "enabled" => Ok(ReasoningType::Enabled),
            "disabled" => Ok(ReasoningType::Disabled),
            other => Err(format!(
                "invalid ReasoningType '{other}', expected one of: unknown, enabled, disabled"
            )),
        }
    }
}

/// A document represented as a JSON object for grounding.
pub type Document = Map<String, Value>;

/// A tool definition available to the model.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    /// Name of the tool.
    pub name: String,
    /// Description of what the tool does.
    pub description: String,
    /// JSON schema for the tool's parameters.
    pub parameters: Map<String, Value>,
}

/// An image reference in message content.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Image {
    /// Placeholder string for the image in the template.
    pub template_placeholder: String,
}

/// Content block within a message.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Content {
    /// Type of this content block.
    #[allow(clippy::struct_field_names)]
    #[serde(rename = "type")]
    pub content_type: ContentType,
    /// Text content (for text type).
    pub text: Option<String>,
    /// Thinking/reasoning content (for thinking type).
    pub thinking: Option<String>,
    /// Image content (for image type).
    pub image: Option<Image>,
    /// Document content as JSON (for document type).
    pub document: Option<Map<String, Value>>,
}

/// A tool call made by the model.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    /// Unique identifier for this tool call.
    pub id: String,
    /// Name of the tool being called.
    pub name: String,
    /// JSON-encoded parameters for the tool call.
    pub parameters: String,
}

/// A message in a conversation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    /// Role of the message sender.
    pub role: Role,
    /// Content blocks in this message.
    #[serde(default)]
    pub content: Vec<Content>,
    /// Tool calls made in this message (for chatbot messages).
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// ID of the tool call this message responds to (for tool messages).
    pub tool_call_id: Option<String>,
    /// Citations in this message.
    #[serde(default)]
    pub citations: Vec<FilterCitation>,
}
