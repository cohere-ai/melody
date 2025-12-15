use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Unknown,
    System,
    User,
    Chatbot,
    Tool,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    Unknown,
    Text,
    Thinking,
    Image,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Unknown => "UNKNOWN",
            ContentType::Text => "TEXT",
            ContentType::Thinking => "THINKING",
            ContentType::Image => "IMAGE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationQuality {
    Unknown,
    Off,
    On,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Grounding {
    Unknown,
    Enabled,
    Disabled,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyMode {
    Unknown,
    None,
    Strict,
    Contextual,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasoningType {
    Unknown,
    Enabled,
    Disabled,
}

impl ReasoningType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReasoningType::Unknown => "UNKNOWN",
            ReasoningType::Enabled => "ENABLED",
            ReasoningType::Disabled => "DISABLED",
        }
    }
}

pub type Document = String;

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Image {
    pub template_placeholder: String,
}

#[derive(Debug, Clone)]
pub struct Content {
    pub content_type: ContentType,
    pub text: Option<String>,
    pub thinking: Option<String>,
    pub image: Option<Image>,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Content>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
    pub additional_fields: BTreeMap<String, serde_json::Value>,
}

