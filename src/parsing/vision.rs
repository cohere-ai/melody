//! Unary parser for parse-/vision-model generations that interleave markdown prose with
//! `[visual_element]…[/visual_element]` blocks.
//!
//! This is intentionally not wired into the streaming [`super::Filter`] path —
//! parse endpoints consume the full generation before mapping to an API response.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const VISUAL_ELEMENT_START: &str = "[visual_element]";
const VISUAL_ELEMENT_END: &str = "[/visual_element]";

/// Field keys recognized inside a `[visual_element]` body (from model generations).
const FIELD_KEYS: &[&str] = &["type", "bbox", "description", "title", "html"];

/// Errors from parsing a vision-model generation.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VisionParseError {
    /// A `[visual_element]` open tag was not closed by `[/visual_element]`.
    #[error("unclosed [visual_element] starting at byte offset {0}")]
    UnclosedElement(usize),
    /// A `bbox` field was present but could not be parsed as four integers.
    #[error("invalid bbox value: {0}")]
    InvalidBBox(String),
}

/// Parsed generation: ordered prose and vision-element segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisionGeneration {
    /// Segments in document order.
    pub segments: Vec<VisionSegment>,
}

/// One piece of a vision generation: markdown prose or a structured element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VisionSegment {
    /// Markdown / plain text outside visual-element tags.
    Text {
        /// Prose content, including surrounding newlines as emitted by the model.
        text: String,
    },
    /// A parsed `[visual_element]` block.
    Element {
        /// Structured fields from the element body.
        element: VisionElement,
    },
}

/// Structured content of a `[visual_element]` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VisionElement {
    /// Element type from the `type:` field (e.g. `table`, `flowchart`).
    #[serde(rename = "type")]
    pub element_type: String,
    /// Axis-aligned bounding box when `bbox:` is present and valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<VisionBBox>,
    /// Model description / annotation when `description:` is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional title (common on tables).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional HTML markup (common on tables).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
}

/// Pixel bounding box: `top_left_x, top_left_y, bottom_right_x, bottom_right_y`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisionBBox {
    /// Left edge (inclusive), in page pixels.
    pub top_left_x: i32,
    /// Top edge (inclusive), in page pixels.
    pub top_left_y: i32,
    /// Right edge (inclusive), in page pixels.
    pub bottom_right_x: i32,
    /// Bottom edge (inclusive), in page pixels.
    pub bottom_right_y: i32,
}

/// Parse a full vision-/parse-model generation into ordered text / element segments.
///
/// Open/close tags are recognized only when they appear alone on a line (optional
/// surrounding whitespace). Field lines are recognized only for known keys:
/// `type`, `bbox`, `description`, `title`, and `html`.
///
/// # Errors
///
/// Returns [`VisionParseError::UnclosedElement`] if a start tag has no matching end,
/// or [`VisionParseError::InvalidBBox`] if a `bbox` field is malformed.
pub fn parse_vision_generation(text: &str) -> Result<VisionGeneration, VisionParseError> {
    let mut segments = Vec::new();
    let mut text_buf = String::new();
    // When inside an element: byte offset of the open tag line, and body so far.
    let mut open: Option<(usize, String)> = None;
    let mut offset = 0;

    for line in text.split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();

        if line_is_tag(line, VISUAL_ELEMENT_END)
            && let Some((_, body)) = open.take()
        {
            segments.push(VisionSegment::Element {
                element: parse_element_body(&body)?,
            });
            continue;
        }

        if let Some((_, body)) = open.as_mut() {
            body.push_str(line);
        } else if line_is_tag(line, VISUAL_ELEMENT_START) {
            if !text_buf.is_empty() {
                segments.push(VisionSegment::Text {
                    text: std::mem::take(&mut text_buf),
                });
            }
            open = Some((line_start, String::new()));
        } else {
            text_buf.push_str(line);
        }
    }

    if let Some((start, _)) = open {
        return Err(VisionParseError::UnclosedElement(start));
    }
    if !text_buf.is_empty() {
        segments.push(VisionSegment::Text { text: text_buf });
    }

    Ok(VisionGeneration { segments })
}

fn line_is_tag(line: &str, tag: &str) -> bool {
    line.trim_end_matches(['\r', '\n']).trim() == tag
}

fn parse_element_body(body: &str) -> Result<VisionElement, VisionParseError> {
    let fields = parse_fields(body);
    let mut element = VisionElement {
        element_type: fields
            .get("type")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        description: fields.get("description").map(|s| s.trim().to_string()),
        title: fields.get("title").map(|s| s.trim().to_string()),
        html: fields.get("html").map(|s| s.trim().to_string()),
        ..Default::default()
    };

    if let Some(bbox_raw) = fields.get("bbox") {
        element.bbox = Some(parse_bbox(bbox_raw)?);
    }

    Ok(element)
}

fn parse_fields(body: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();

    for line in body.lines() {
        if let Some((key, rest)) = split_field_line(line) {
            if let Some(prev_key) = current_key.take() {
                fields.insert(prev_key, std::mem::take(&mut current_value));
            }
            current_key = Some(key);
            current_value = rest;
        } else if current_key.is_some() {
            current_value.push('\n');
            current_value.push_str(line);
        }
    }

    if let Some(prev_key) = current_key {
        fields.insert(prev_key, current_value);
    }

    fields
}

/// A field line is `key: optional_rest` where `key` is one of [`FIELD_KEYS`].
fn split_field_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    let colon = trimmed.find(':')?;
    let key = &trimmed[..colon];
    if !FIELD_KEYS.contains(&key) {
        return None;
    }
    let rest = trimmed[colon + 1..].trim_start().to_string();
    Some((key.to_string(), rest))
}

fn parse_bbox(raw: &str) -> Result<VisionBBox, VisionParseError> {
    let parts: Vec<&str> = raw
        .trim()
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() != 4 {
        return Err(VisionParseError::InvalidBBox(raw.trim().to_string()));
    }
    let parse_i32 = |s: &str| {
        s.parse::<i32>()
            .map_err(|_| VisionParseError::InvalidBBox(raw.trim().to_string()))
    };
    Ok(VisionBBox {
        top_left_x: parse_i32(parts[0])?,
        top_left_y: parse_i32(parts[1])?,
        bottom_right_x: parse_i32(parts[2])?,
        bottom_right_y: parse_i32(parts[3])?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const SAMPLE: &str = r#"[visual_element]

type: flowchart

bbox: 183,87,812,299

description: The diagram shows a multi-stage process starting with Image Patches and RoPE-2D inputs. The patches are processed through a Block-diagonal attention mask and a Bidirectional transformer to produce Output Embeddings. These embeddings are fed into a Vision-Language Projector, which also receives input from a Vision-Language Projector (MLP). The architecture includes specific components like a Vision-Language Projector, a Bidirectional transformer, and a Vision-Language Projector. The diagram also highlights the addition of [IMG_BREAK] and [IMG_END] tokens to the embeddings. The outputs are shown as various image patches and embeddings, with specific attention masks and position encodings ($\theta_1, \theta_2, \theta_3, \theta_4$) indicated.

[/visual_element]

**Figure 2: Pixtral Vision Encoder.** Pixtral uses a new vision encoder, which is trained from scratch to natively support variable image sizes and aspect ratios. Block-diagonal attention masks enable sequence packing for batching, while RoPE-2D encodings facilitate variable image sizes. Note that the attention mask and position encodings are fed to the vision transformer as additional input, and utilized only in the self-attention layers.

## 2 Architectural details

Pixtral 12B is based on the transformer architecture [22], and consists of a *multimodal decoder* to perform high-level reasoning, and a *vision encoder* to allow the model to ingest images. The main parameters of the model are summarized in Table 1.

### 2.1 Multimodal Decoder

Pixtral 12B is built on top of Mistral Nemo 12B [15], a 12-billion parameter decoder-only language model that achieves strong performance across a range of knowledge and reasoning tasks.
Table 1: Decoder and encoder parameters.
[visual_element]

type: table

bbox: 566,413,816,556

title: Table 1: Decoder and encoder parameters.

description: The table presents the parameter counts for the Decoder and Encoder of the Pixtral 12B model. It includes dimensions such as dim (5120 for Decoder, 1024 for Encoder), n_layers (40 vs 24), head_dim (128 vs 64), and hidden_dim (14336 vs 4096). It also lists the number of heads (32 vs 16), context_length (131072 vs 4096), and patch_size (16 for Encoder). The decoder has a vocab_size of 131072, while the encoder does not have a specified vocab_size.

html: <table><thead><tr><td>Parameters</td><td>Decoder</td><td>Encoder</td></tr></thead><tbody><tr><td>dim</td><td>5120</td><td>1024</td></tr><tr><td>n_layers</td><td>40</td><td>24</td></tr><tr><td>head_dim</td><td>128</td><td>64</td></tr><tr><td>hidden_dim</td><td>14336</td><td>4096</td></tr><tr><td>n_heads</td><td>32</td><td>16</td></tr><tr><td>n_kv_heads</td><td>8</td><td>16</td></tr><tr><td>context_len</td><td>131072</td><td>4096</td></tr><tr><td>vocab_size</td><td>131072</td><td>-</td></tr><tr><td>patch_size</td><td>-</td><td>16</td></tr></tbody></table>

[/visual_element]

### 2.2 Vision Encoder
"#;

    #[test]
    fn parses_pixtral_sample() {
        let parsed = parse_vision_generation(SAMPLE).unwrap();
        assert_eq!(parsed.segments.len(), 4);

        match &parsed.segments[0] {
            VisionSegment::Element { element } => {
                assert_eq!(element.element_type, "flowchart");
                assert_eq!(
                    element.bbox,
                    Some(VisionBBox {
                        top_left_x: 183,
                        top_left_y: 87,
                        bottom_right_x: 812,
                        bottom_right_y: 299,
                    })
                );
                assert!(
                    element
                        .description
                        .as_ref()
                        .unwrap()
                        .contains("Image Patches")
                );
                assert!(element.html.is_none());
                assert!(element.title.is_none());
            }
            other => panic!("expected flowchart element, got {other:?}"),
        }

        match &parsed.segments[1] {
            VisionSegment::Text { text } => {
                assert!(text.contains("**Figure 2: Pixtral Vision Encoder.**"));
                assert!(text.contains("### 2.1 Multimodal Decoder"));
            }
            other => panic!("expected prose, got {other:?}"),
        }

        match &parsed.segments[2] {
            VisionSegment::Element { element } => {
                assert_eq!(element.element_type, "table");
                assert_eq!(
                    element.bbox,
                    Some(VisionBBox {
                        top_left_x: 566,
                        top_left_y: 413,
                        bottom_right_x: 816,
                        bottom_right_y: 556,
                    })
                );
                assert_eq!(
                    element.title.as_deref(),
                    Some("Table 1: Decoder and encoder parameters.")
                );
                assert!(element.html.as_ref().unwrap().starts_with("<table>"));
                assert!(element.html.as_ref().unwrap().contains("<td>5120</td>"));
            }
            other => panic!("expected table element, got {other:?}"),
        }

        match &parsed.segments[3] {
            VisionSegment::Text { text } => {
                assert!(text.contains("### 2.2 Vision Encoder"));
            }
            other => panic!("expected trailing prose, got {other:?}"),
        }
    }

    #[test]
    fn prose_only() {
        let parsed = parse_vision_generation("just markdown\n\n## heading\n").unwrap();
        assert_eq!(
            parsed.segments,
            vec![VisionSegment::Text {
                text: "just markdown\n\n## heading\n".into(),
            }]
        );
    }

    #[test]
    fn unclosed_element_errors() {
        let err = parse_vision_generation("[visual_element]\ntype: table\n").unwrap_err();
        assert_eq!(err, VisionParseError::UnclosedElement(0));
    }

    #[test]
    fn invalid_bbox_errors() {
        let text = "[visual_element]\nbbox: 1,2,3\n[/visual_element]";
        let err = parse_vision_generation(text).unwrap_err();
        assert!(matches!(err, VisionParseError::InvalidBBox(_)));
    }

    #[test]
    fn multiline_description() {
        let text = "\
[visual_element]
type: image
description: line one
line two
still description
bbox: 0,0,10,10
[/visual_element]
";
        let parsed = parse_vision_generation(text).unwrap();
        match &parsed.segments[0] {
            VisionSegment::Element { element } => {
                assert_eq!(
                    element.description.as_deref(),
                    Some("line one\nline two\nstill description")
                );
                assert_eq!(
                    element.bbox,
                    Some(VisionBBox {
                        top_left_x: 0,
                        top_left_y: 0,
                        bottom_right_x: 10,
                        bottom_right_y: 10,
                    })
                );
            }
            other => panic!("expected element, got {other:?}"),
        }
    }

    #[test]
    fn unknown_field_keys_continue_current_value() {
        let text = "\
[visual_element]
type: other
confidence: 0.9
description: line with confidence: still prose
[/visual_element]";
        let parsed = parse_vision_generation(text).unwrap();
        match &parsed.segments[0] {
            VisionSegment::Element { element } => {
                assert_eq!(element.element_type, "other\nconfidence: 0.9");
                assert_eq!(
                    element.description.as_deref(),
                    Some("line with confidence: still prose")
                );
            }
            other => panic!("expected element, got {other:?}"),
        }
    }

    #[test]
    fn inline_tags_are_not_delimiters() {
        let text = "\
see [visual_element] inline
[visual_element]
type: table
description: mentions [/visual_element] mid-line
[/visual_element]
and [visual_element] again
";
        let parsed = parse_vision_generation(text).unwrap();
        assert_eq!(parsed.segments.len(), 3);
        match &parsed.segments[0] {
            VisionSegment::Text { text } => {
                assert!(text.contains("see [visual_element] inline"));
            }
            other => panic!("expected leading text, got {other:?}"),
        }
        match &parsed.segments[1] {
            VisionSegment::Element { element } => {
                assert_eq!(element.element_type, "table");
                assert_eq!(
                    element.description.as_deref(),
                    Some("mentions [/visual_element] mid-line")
                );
            }
            other => panic!("expected element, got {other:?}"),
        }
        match &parsed.segments[2] {
            VisionSegment::Text { text } => {
                assert!(text.contains("and [visual_element] again"));
            }
            other => panic!("expected trailing text, got {other:?}"),
        }
    }

    #[test]
    fn end_tag_inside_description_on_own_line_still_closes() {
        // Standalone end tag always closes — even if it appears where a field value continues.
        let text = "\
[visual_element]
type: table
description: before
[/visual_element]
after
";
        let parsed = parse_vision_generation(text).unwrap();
        assert_eq!(parsed.segments.len(), 2);
        match &parsed.segments[0] {
            VisionSegment::Element { element } => {
                assert_eq!(element.description.as_deref(), Some("before"));
            }
            other => panic!("expected element, got {other:?}"),
        }
    }

    #[test]
    fn round_trips_through_json() {
        let parsed = parse_vision_generation(SAMPLE).unwrap();
        let json = serde_json::to_string(&parsed).unwrap();
        let back: VisionGeneration = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, back);
    }
}
