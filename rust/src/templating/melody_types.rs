use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use super::ordered_json::OrderedJson;

pub type Document = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: OrderedJson,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Content>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub tool_call_id: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub additional_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    #[serde(rename = "type")]
    pub content_type: ContentType,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub thinking: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub template_placeholder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub parameters: OrderedJson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "chatbot")]
    Chatbot,
    #[serde(rename = "tool")]
    Tool,
}

impl Role {
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "system" => Ok(Role::System),
            "user" => Ok(Role::User),
            "chatbot" => Ok(Role::Chatbot),
            "tool" => Ok(Role::Tool),
            _ => Err(format!("unknown role: {}", s)),
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Role::Unknown => "UNKNOWN",
            Role::System => "SYSTEM",
            Role::User => "USER",
            Role::Chatbot => "CHATBOT",
            Role::Tool => "TOOL",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "thinking")]
    Thinking,
    #[serde(rename = "image_url")]
    ImageUrl,
}

impl ContentType {
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "text" => Ok(ContentType::Text),
            "thinking" => Ok(ContentType::Thinking),
            "image_url" => Ok(ContentType::ImageUrl),
            _ => Err(format!("unknown content type: {}", s)),
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ContentType::Unknown => "UNKNOWN",
            ContentType::Text => "TEXT",
            ContentType::Thinking => "THINKING",
            ContentType::ImageUrl => "IMAGE_URL",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CitationQuality {
    #[serde(rename = "unknown")]
    #[default]
    Unknown,
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "on")]
    On,
}

impl CitationQuality {
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "off" => Ok(CitationQuality::Off),
            "on" => Ok(CitationQuality::On),
            _ => Err(format!("unknown citation quality: {}", s)),
        }
    }
}

impl fmt::Display for CitationQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CitationQuality::Unknown => "UNKNOWN",
            CitationQuality::Off => "OFF",
            CitationQuality::On => "ON",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Grounding {
    #[serde(rename = "unknown")]
    #[default]
    Unknown,
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
}

impl Grounding {
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "enabled" => Ok(Grounding::Enabled),
            "disabled" => Ok(Grounding::Disabled),
            _ => Err(format!("unknown grounding value: {}", s)),
        }
    }
}

impl fmt::Display for Grounding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Grounding::Unknown => "UNKNOWN",
            Grounding::Enabled => "ENABLED",
            Grounding::Disabled => "DISABLED",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SafetyMode {
    #[serde(rename = "unknown")]
    #[default]
    Unknown,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "strict")]
    Strict,
    #[serde(rename = "contextual")]
    Contextual,
}

impl SafetyMode {
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "none" => Ok(SafetyMode::None),
            "strict" => Ok(SafetyMode::Strict),
            "contextual" => Ok(SafetyMode::Contextual),
            _ => Err(format!("unknown safety mode: {}", s)),
        }
    }
}

impl fmt::Display for SafetyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SafetyMode::Unknown => "UNKNOWN",
            SafetyMode::None => "NONE",
            SafetyMode::Strict => "STRICT",
            SafetyMode::Contextual => "CONTEXTUAL",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ReasoningType {
    #[serde(rename = "unknown")]
    #[default]
    Unknown,
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
}

impl ReasoningType {
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "enabled" => Ok(ReasoningType::Enabled),
            "disabled" => Ok(ReasoningType::Disabled),
            _ => Err(format!("unknown reasoning type: {}", s)),
        }
    }
}

impl fmt::Display for ReasoningType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ReasoningType::Unknown => "UNKNOWN",
            ReasoningType::Enabled => "ENABLED",
            ReasoningType::Disabled => "DISABLED",
        };
        write!(f, "{}", s)
    }
}
