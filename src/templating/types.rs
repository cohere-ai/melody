use serde::Deserialize;
use serde_json::{Map, Value};

use crate::FilterCitation;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Role {
    Unknown,
    System,
    User,
    Chatbot,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum ContentType {
    Unknown,
    Text,
    Thinking,
    Image,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum CitationQuality {
    Unknown,
    Off,
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
    pub fn as_str(&self) -> &'static str {
        match self {
            CitationQuality::Unknown => "UNKNOWN",
            CitationQuality::Off => "OFF",
            CitationQuality::On => "ON",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Grounding {
    Unknown,
    Enabled,
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
    pub fn as_str(&self) -> &'static str {
        match self {
            Grounding::Unknown => "UNKNOWN",
            Grounding::Enabled => "ENABLED",
            Grounding::Disabled => "DISABLED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum SafetyMode {
    Unknown,
    None,
    Strict,
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
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyMode::Unknown => "UNKNOWN",
            SafetyMode::None => "NONE",
            SafetyMode::Strict => "STRICT",
            SafetyMode::Contextual => "CONTEXTUAL",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum ReasoningType {
    Unknown,
    Enabled,
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

pub type Document = Map<String, Value>;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Image {
    pub template_placeholder: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Content {
    #[allow(clippy::struct_field_names)]
    #[serde(rename = "type")]
    pub content_type: ContentType,
    pub text: Option<String>,
    pub thinking: Option<String>,
    pub image: Option<Image>,
    pub document: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub parameters: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub role: Role,
    #[serde(default)]
    pub content: Vec<Content>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub citations: Vec<FilterCitation>,
}
