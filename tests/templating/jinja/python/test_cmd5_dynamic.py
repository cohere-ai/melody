"""Render cmd5-dynamic like Hugging Face / vLLM, with caller-supplied wording."""

import itertools
import json
from os.path import join
from typing import Any

import pytest

from test_jinja import Engine, render_template

TEMPLATE_DIR = "templates/jinja"
CMD5_DYNAMIC_CASES_DIR = "../cmd5_dynamic"
ENGINES = [Engine.JINJA2, Engine.MINIJINJA]


def tool(name: str, description: str, param: str) -> dict[str, Any]:
    return {
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": {
                "type": "object",
                "properties": {param: {"type": "string"}},
                "required": [param],
            },
        },
    }


SEARCH = tool("search", "Search the web.", "query")
GET_WEATHER = tool("get_weather", "Get the current weather for a city.", "city")

BASE: dict[str, Any] = {
    "platform_instruction_override": "You are Command, built by Cohere.",
    "grounding": "enabled",
    "reasoning": True,
    "tools": [GET_WEATHER, SEARCH],
    "developer_instruction": "Only answer questions about weather.",
    "messages": [{"role": "user", "content": "What's the weather in Toronto?"}],
    "add_generation_prompt": True,
    "bos_token": "<BOS_TOKEN>",
}


def served_grounding_sentence() -> str:
    with open(join(TEMPLATE_DIR, "cmd5.jinja")) as f:
        return next(
            line for line in f.read().splitlines() if line.startswith("Note that both")
        )


SERVED_FIELDS = {
    "wording": {"grounding_clause": served_grounding_sentence()},
    "layout": [["platform", "grounding", "tools", "json_mode"]],
    "show_tools_header": True,
}


@pytest.mark.parametrize("engine", ENGINES)
def test_served_wording_matches_cmd5(engine: Engine) -> None:
    variations = itertools.product(
        [None, "You are Command."],
        ["enabled", "disabled"],
        [[], [GET_WEATHER, SEARCH]],
        [[], [{"data": {"title": "t", "snippet": "s"}}]],
        [
            None,
            {"type": "json_object"},
            {"type": "json_object", "schema": '{"type": "object"}'},
        ],
        [None, "Be brief."],
    )
    for platform, grounding, tools, documents, response_format, developer in variations:
        kwargs = {
            **BASE,
            "platform_instruction_override": platform,
            "grounding": grounding,
            "tools": tools,
            "documents": documents,
            "response_format": response_format,
            "developer_instruction": developer,
        }
        served = render_template(TEMPLATE_DIR, "cmd5.jinja", engine, **kwargs)
        dynamic = render_template(
            TEMPLATE_DIR, "cmd5-dynamic.jinja", engine, **kwargs, **SERVED_FIELDS
        )
        assert dynamic == served, kwargs


@pytest.mark.parametrize("engine", ENGINES)
def test_openai_inputs_match_native_fixture(engine: Engine) -> None:
    case_dir = join(CMD5_DYNAMIC_CASES_DIR, "split_turns_custom_grounding")
    with open(join(case_dir, "input.json")) as f:
        native = json.load(f)
    rendered = render_template(
        TEMPLATE_DIR,
        "cmd5-dynamic.jinja",
        engine,
        **{**BASE, "tools": [SEARCH, GET_WEATHER]},
        **native["additional_template_fields"],
    )
    with open(join(case_dir, "output.txt")) as f:
        assert rendered == f.read()


WORDING = {"grounding_clause": "Cite your sources."}


@pytest.mark.parametrize("engine", ENGINES)
@pytest.mark.parametrize(
    "fields, overrides, message",
    [
        (
            {"layout": [["tools"]], "show_tools_header": True},
            {},
            "cmd5-dynamic needs wording, layout and show_tools_header",
        ),
        (
            {
                "wording": {},
                "layout": [["platform", "grounding", "tools"]],
                "show_tools_header": True,
            },
            {},
            "Missing cmd5-dynamic input: wording.grounding_clause",
        ),
        (
            {
                "wording": WORDING,
                "layout": [["platform", "grounding", "tool"]],
                "show_tools_header": True,
            },
            {},
            "Unknown section in layout: tool",
        ),
        (
            {
                "wording": WORDING,
                "layout": [["platform", "grounding"], ["tools", "platform"]],
                "show_tools_header": True,
            },
            {},
            "Section appears more than once in layout: platform",
        ),
        (
            {
                "wording": WORDING,
                "layout": [["platform", "tools"]],
                "show_tools_header": True,
            },
            {},
            "Section has content but is missing from layout: grounding",
        ),
        (
            {"wording": WORDING, "layout": [], "show_tools_header": True},
            {},
            "Section has content but is missing from layout: platform",
        ),
        (
            {
                "wording": WORDING,
                "layout": [["platform", "grounding", "tools"]],
                "show_tools_header": True,
            },
            {"response_format": {"type": "json_object"}},
            "Section has content but is missing from layout: json_mode",
        ),
    ],
)
def test_bad_inputs_raise(
    engine: Engine, fields: dict[str, Any], overrides: dict[str, Any], message: str
) -> None:
    with pytest.raises(Exception, match=message):
        render_template(
            TEMPLATE_DIR,
            "cmd5-dynamic.jinja",
            engine,
            **{**BASE, **overrides},
            **fields,
        )


@pytest.mark.parametrize("engine", ENGINES)
def test_sections_without_content_are_skipped(engine: Engine) -> None:
    rendered = render_template(
        TEMPLATE_DIR,
        "cmd5-dynamic.jinja",
        engine,
        **{**BASE, "platform_instruction_override": None, "grounding": "disabled"},
        wording={},
        layout=[["platform", "grounding"], ["tools", "json_mode"]],
        show_tools_header=False,
    )
    assert rendered.startswith(
        "<BOS_TOKEN><|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|><|START_TEXT|>```json"
    )
    assert rendered.count("<|START_TEXT|>```json") == 1
    assert "You are Command" not in rendered
