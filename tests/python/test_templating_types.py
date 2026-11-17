"""Tests for the typed render configs in cohere_melody.templating."""

import json
import re
from pathlib import Path
from typing import get_args

import pytest
from cohere_melody import (
    Citation,
    Message,
    RenderCmd3Options,
    RenderCmd4Options,
    RenderCmd5Options,
    Tool,
    render_cmd3,
    render_cmd4,
    render_cmd5,
)
from cohere_melody import templating

REPO_ROOT = Path(__file__).resolve().parents[2]


def rust_struct_fields(path: str, struct: str) -> dict[str, bool]:
    """Map each deserialized field of a Rust struct to whether it is required."""
    source = (REPO_ROOT / path).read_text()
    match = re.search(
        rf"((?:#\[[^\]]*\]\s*)*)pub struct {struct}\b[^{{]*\{{(.*?)\n\}}", source, re.S
    )
    assert match, f"struct {struct} not found in {path}"
    struct_default = "#[serde(default)]" in match.group(1)
    fields = {}
    for attrs, name, rust_type in re.findall(
        r"((?:\s*#\[[^\]]*\])*)\s*pub (\w+): ([^\n]+),", match.group(2)
    ):
        if "serde(skip" in attrs:
            continue
        rename = re.search(r'rename = "(\w+)"', attrs)
        has_default = struct_default or "serde(default" in attrs
        required = not has_default and not rust_type.startswith("Option<")
        fields[rename.group(1) if rename else name] = required
    return fields


def rust_enum_values(enum: str) -> set[str]:
    source = (REPO_ROOT / "src/templating/types.rs").read_text()
    match = re.search(rf"impl TryFrom<String> for {enum} \{{(.*?)\n\}}", source, re.S)
    assert match, f"enum {enum} not found"
    return set(re.findall(rf'"(\w+)" => Ok\({enum}::', match.group(1)))


def typed_dict_fields(typed_dict) -> dict[str, bool]:
    keys = typed_dict.__required_keys__ | typed_dict.__optional_keys__
    return {key: key in typed_dict.__required_keys__ for key in keys}


@pytest.mark.parametrize(
    ("typed_dict", "path", "struct"),
    [
        (templating.RenderCmd3Options, "src/templating/lib.rs", "RenderCmd3Options"),
        (templating.RenderCmd4Options, "src/templating/lib.rs", "RenderCmd4Options"),
        (templating.Message, "src/templating/types.rs", "Message"),
        (templating.Content, "src/templating/types.rs", "Content"),
        (templating.Image, "src/templating/types.rs", "Image"),
        (templating.Tool, "src/templating/types.rs", "Tool"),
        (templating.ToolCall, "src/templating/types.rs", "ToolCall"),
        (templating.Citation, "src/parsing/types.rs", "FilterCitation"),
        (templating.CitationSource, "src/parsing/types.rs", "Source"),
    ],
)
def test_typed_dicts_match_rust_structs(typed_dict, path, struct):
    assert typed_dict_fields(typed_dict) == rust_struct_fields(path, struct)


@pytest.mark.parametrize(
    ("literal", "enum"),
    [
        (templating.Role, "Role"),
        (templating.ContentType, "ContentType"),
        (templating.Grounding, "Grounding"),
        (templating.ReasoningType, "ReasoningType"),
        (templating.SafetyMode, "SafetyMode"),
        (templating.CitationQuality, "CitationQuality"),
    ],
)
def test_literals_match_rust_enums(literal, enum):
    # "unknown" is accepted by Melody but renders nothing useful, so it is left out.
    assert set(get_args(literal)) == rust_enum_values(enum) - {"unknown"}


def test_cmd5_options_are_cmd4_options():
    assert RenderCmd5Options is RenderCmd4Options


SEARCH: Tool = {
    "name": "search",
    "description": "Search the web.",
    "parameters": {"type": "object", "properties": {"query": {"type": "string"}}},
}

CITATION: Citation = {
    "start_index": 0,
    "end_index": 5,
    "text": "Paris",
    "is_thinking": False,
    "sources": [{"tool_call_index": 0, "tool_result_indices": [0]}],
}

MESSAGES: list[Message] = [
    {"role": "system", "content": [{"type": "text", "text": "Be brief."}]},
    {
        "role": "user",
        "content": [
            {"type": "text", "text": "Where is the Eiffel Tower?"},
            {"type": "image", "image": {"template_placeholder": "<|IMG|>"}},
        ],
    },
    {
        "role": "chatbot",
        "content": [{"type": "thinking", "thinking": "I will search."}],
        "tool_calls": [
            {
                "id": "call_0",
                "name": "search",
                "parameters": json.dumps({"query": "Eiffel Tower"}),
            }
        ],
    },
    {
        "role": "tool",
        "tool_call_id": "call_0",
        "content": [{"type": "document", "document": {"snippet": "Paris, France"}}],
    },
    {
        "role": "chatbot",
        "content": [{"type": "text", "text": "Paris."}],
        "citations": [CITATION],
    },
    {"role": "user", "content": [{"type": "text", "text": "Thanks"}]},
]


def test_render_cmd5_with_every_option():
    options: RenderCmd5Options = {
        "messages": MESSAGES,
        "template_id": "cmd5",
        "template": "",
        "template_jinja": "",
        "use_jinja": True,
        "dev_instruction": "Answer briefly.",
        "platform_instruction": "You are a test model.",
        "documents": [{"title": "Doc", "snippet": "A document."}],
        "available_tools": [SEARCH],
        "grounding": "enabled",
        "reasoning_type": "enabled",
        "response_prefix": None,
        "json_schema": None,
        "json_mode": False,
        "additional_template_fields": {},
        "escaped_special_tokens": {},
    }
    assert set(options) == set(RenderCmd5Options.__optional_keys__)
    rendered = render_cmd5(options)
    assert "You are a test model." in rendered
    assert "Answer briefly." in rendered
    assert "<co>Paris</co: 0:[0]>." in rendered


def test_render_cmd4_with_every_option():
    options: RenderCmd4Options = {
        "messages": MESSAGES,
        "template_id": "cmd4-v1",
        "template": "",
        "template_jinja": "",
        "use_jinja": True,
        "dev_instruction": "Answer briefly.",
        "platform_instruction": "You are a test model.",
        "documents": [{"title": "Doc", "snippet": "A document."}],
        "available_tools": [SEARCH],
        "grounding": "enabled",
        "reasoning_type": "enabled",
        "response_prefix": None,
        "json_schema": None,
        "json_mode": False,
        "additional_template_fields": {},
        "escaped_special_tokens": {},
    }
    assert set(options) == set(RenderCmd4Options.__optional_keys__)
    rendered = render_cmd4(options)
    assert "Answer briefly." in rendered
    assert "Paris" in rendered


def test_render_cmd3_with_every_option():
    options: RenderCmd3Options = {
        "messages": [
            {"role": "user", "content": [{"type": "text", "text": "Hi"}]},
        ],
        "template_id": "cmd3-v1",
        "template": "",
        "template_jinja": "",
        "use_jinja": True,
        "dev_instruction": "Answer briefly.",
        "documents": [{"title": "Doc", "snippet": "A document."}],
        "available_tools": [SEARCH],
        "safety_mode": "contextual",
        "citation_quality": "on",
        "reasoning_type": "disabled",
        "skip_preamble": False,
        "response_prefix": None,
        "json_schema": None,
        "json_mode": False,
        "additional_template_fields": {},
        "escaped_special_tokens": {},
    }
    assert set(options) == set(RenderCmd3Options.__optional_keys__)
    rendered = render_cmd3(options)
    assert "Answer briefly." in rendered
    assert "Hi" in rendered


def test_plain_dicts_still_render():
    config = {
        "messages": [{"role": "USER", "content": [{"type": "TEXT", "text": "Hi"}]}]
    }
    assert render_cmd5(config) == render_cmd5(
        {"messages": [{"role": "user", "content": [{"type": "text", "text": "Hi"}]}]}
    )


def test_unknown_key_is_rejected():
    with pytest.raises(ValueError, match="unknown field `dev_instructions`"):
        render_cmd5({"messages": [], "dev_instructions": "typo"})
