"""Tests for rendering AgentTrajectory data (cohere_melody.agent_trajectory)."""

import copy
import re

import pytest
from cohere_melody import render_cmd5
from cohere_melody.agent_trajectory import (
    UnsupportedTrajectoryError,
    render_trajectory_cmd4,
    render_trajectory_cmd5,
    trajectory_to_render_options,
)

SEARCH_TOOL = {
    "name": "search",
    "description": "Search the web.",
    "parameters": {
        "type": "object",
        "properties": {"query": {"type": "string"}},
        "required": ["query"],
    },
    "responses": None,
}

IMAGE = {
    "image_url": "data:image/png;base64,iVBORw0KGgo=",
    "mime_type": "image/png",
    "language_id": None,
    "metadata": {"width": 4, "height": 4},
}


def text(value, grounding_spans=None, content_type="text"):
    return {
        "content_type": content_type,
        "text": value,
        "language_id": None,
        "grounding_spans": grounding_spans,
        "metadata": None,
    }


def reasoning(value, grounding_spans=None):
    return text(value, grounding_spans, content_type="reasoning")


def image():
    return {"content_type": "image", **IMAGE}


def tool_calls(*calls):
    return {
        "content_type": "tool_calls",
        "tool_calls": [
            {"id": call_id, "name": "search", "parameters": params, "metadata": None}
            for call_id, params in calls
        ],
        "metadata": None,
    }


def tool_results(*results):
    return {
        "content_type": "tool_results",
        "tool_results": [
            {
                "tool_call_id": call_id,
                "results": items,
                "is_error": {"missing": "MISSING"},
                "metadata": None,
            }
            for call_id, items in results
        ],
        "metadata": None,
    }


def span(start, end, *result_ids):
    return {
        "start": start,
        "end": end,
        "tool_result_ids": [
            {"tool_call_id": call_id, "index": index} for call_id, index in result_ids
        ],
    }


def turn(role, *contents):
    return {"role": role, "contents": list(contents), "metadata": None}


def structured_preamble(
    tools=(), reasoning_effort="ON", grounding_style="SPAN", **sections
):
    preamble = {
        "platform_preamble": None,
        "safety_preamble": {"mode": "contextual"},
        "default_preamble": {"mode": "interactive"},
        "developer_preamble": None,
        "react_config": {
            "tools": list(tools),
            "reasoning_effort": reasoning_effort,
            "grounding_style": grounding_style,
        },
        "current_timestamp": None,
        "reasoning_clause_in_default_preamble": False,
    }
    preamble.update(sections)
    return preamble


def trajectory(*turns, preamble=None):
    return {
        "id": "00000000-0000-0000-0000-000000000000",
        "source": "test",
        "split": "test",
        "version": "0.0.0",
        "metadata": None,
        "preamble": preamble,
        "turns": list(turns),
    }


ANSWER = "Paris is in France. Lyon too."


def tool_use_trajectory(grounding_style="SPAN"):
    return trajectory(
        turn("User", text("Where are Paris and Lyon?")),
        turn(
            "Chatbot",
            reasoning("I will search."),
            tool_calls(("call_a", {"query": "paris"}), ("call_b", {"query": "lyon"})),
        ),
        turn(
            "Tool",
            tool_results(
                ("call_a", [{"title": "Paris", "snippet": "Paris, France"}]),
                ("call_b", [{"title": "Lyon 1"}, {"title": "Lyon 2"}]),
            ),
        ),
        turn(
            "Chatbot",
            reasoning("Both found.", [span(0, 4, ("call_b", 1))]),
            text(
                ANSWER,
                [span(0, 19, ("call_b", 1), ("call_a", 0), ("call_b", 0))],
            ),
        ),
        preamble=structured_preamble(
            tools=[SEARCH_TOOL],
            grounding_style=grounding_style,
            platform_preamble={"content": "You are Command."},
            developer_preamble={"content": "Answer briefly."},
        ),
    )


class TestRenderOptions:
    def test_tool_use_trajectory(self):
        options = trajectory_to_render_options(tool_use_trajectory())
        assert options == {
            "platform_instruction": "You are Command.",
            "dev_instruction": "Answer briefly.",
            "available_tools": [
                {
                    "name": "search",
                    "description": "Search the web.",
                    "parameters": SEARCH_TOOL["parameters"],
                }
            ],
            "reasoning_type": "enabled",
            "grounding": "enabled",
            "messages": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": "Where are Paris and Lyon?"}],
                },
                {
                    "role": "chatbot",
                    "content": [{"type": "thinking", "thinking": "I will search."}],
                    "tool_calls": [
                        {
                            "id": "call_a",
                            "name": "search",
                            "parameters": '{"query": "paris"}',
                        },
                        {
                            "id": "call_b",
                            "name": "search",
                            "parameters": '{"query": "lyon"}',
                        },
                    ],
                    "citations": [],
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_a",
                    "content": [
                        {
                            "type": "document",
                            "document": {"title": "Paris", "snippet": "Paris, France"},
                        }
                    ],
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_b",
                    "content": [
                        {"type": "document", "document": {"title": "Lyon 1"}},
                        {"type": "document", "document": {"title": "Lyon 2"}},
                    ],
                },
                {
                    "role": "chatbot",
                    "content": [
                        {"type": "thinking", "thinking": "Both found."},
                        {"type": "text", "text": ANSWER},
                    ],
                    "tool_calls": [],
                    "citations": [
                        {
                            "start_index": 0,
                            "end_index": 4,
                            "text": "Both",
                            "is_thinking": True,
                            "sources": [
                                {"tool_call_index": 1, "tool_result_indices": [1]}
                            ],
                        },
                        {
                            "start_index": 0,
                            "end_index": 19,
                            "text": "Paris is in France.",
                            "is_thinking": False,
                            "sources": [
                                {"tool_call_index": 0, "tool_result_indices": [0]},
                                {"tool_call_index": 1, "tool_result_indices": [0, 1]},
                            ],
                        },
                    ],
                },
            ],
        }

    def test_unstructured_preamble_becomes_dev_instruction(self):
        options = trajectory_to_render_options(
            trajectory(turn("User", text("Hi")), preamble={"content": "Be nice."})
        )
        assert options["dev_instruction"] == "Be nice."
        assert "platform_instruction" not in options
        assert options["reasoning_type"] == "disabled"
        assert options["grounding"] == "disabled"
        assert options["available_tools"] == []

    def test_null_preamble(self):
        options = trajectory_to_render_options(trajectory(turn("User", text("Hi"))))
        assert "dev_instruction" not in options
        assert options["reasoning_type"] == "disabled"

    def test_grounding_off_drops_citations(self):
        options = trajectory_to_render_options(tool_use_trajectory("OFF"))
        assert options["grounding"] == "disabled"
        assert all(not m.get("citations") for m in options["messages"])

    def test_accepts_model_with_model_dump(self):
        data = tool_use_trajectory()

        class Model:
            def model_dump(self, mode):
                assert mode == "json"
                return data

        assert trajectory_to_render_options(Model()) == trajectory_to_render_options(
            data
        )

    def test_rejects_other_types(self):
        with pytest.raises(TypeError):
            trajectory_to_render_options(["not", "a", "trajectory"])

    def test_does_not_mutate_input(self):
        data = tool_use_trajectory()
        before = copy.deepcopy(data)
        trajectory_to_render_options(data)
        assert data == before


class TestRender:
    def test_cmd5_matches_equivalent_melody_request(self):
        data = tool_use_trajectory()
        rendered = render_trajectory_cmd5(data)
        assert rendered == render_cmd5(trajectory_to_render_options(data))

    def test_cmd5_tool_use(self):
        rendered = render_trajectory_cmd5(tool_use_trajectory())
        assert rendered.startswith(
            "<BOS_TOKEN><|START_OF_TURN_TOKEN|><|SYSTEM_TOKEN|><|START_TEXT|>"
            "You are Command."
        )
        assert '"name": "search"' in rendered
        assert "<|START_TEXT|>Answer briefly.<|END_TEXT|>" in rendered
        assert (
            '<cofl:tool_call id="0" name="search">'
            '<cofl:value name="query" type="raw">paris</cofl:value></cofl:tool_call>'
            '<cofl:tool_call id="1" name="search">'
        ) in rendered
        assert (
            '<cofl:tool_result id="1" is_error="false">'
            '<cofl:tool_result_item index="0">{"title": "Lyon 1"}</cofl:tool_result_item>'
            '<cofl:tool_result_item index="1">{"title": "Lyon 2"}</cofl:tool_result_item>'
        ) in rendered
        assert (
            "<|START_THINKING|><co>Both</co: 1:[1]> found.<|END_THINKING|>" in rendered
        )
        assert (
            "<|START_TEXT|><co>Paris is in France.</co: 0:[0],1:[0,1]> Lyon too."
            "<|END_TEXT|>"
        ) in rendered
        assert rendered.endswith(
            "<|START_OF_TURN_TOKEN|><|CHATBOT_TOKEN|><|START_THINKING|>"
        )

    def test_cmd5_reasoning_off(self):
        data = trajectory(
            turn("User", text("Hi")),
            turn("Chatbot", text("Hello!")),
            preamble=structured_preamble(reasoning_effort="OFF", grounding_style="OFF"),
        )
        rendered = render_trajectory_cmd5(data)
        assert (
            "<|CHATBOT_TOKEN|><|START_THINKING|><|END_THINKING|>"
            "<|START_TEXT|>Hello!<|END_TEXT|>"
        ) in rendered
        assert rendered.endswith("<|START_THINKING|><|END_THINKING|>")

    def test_cmd5_template_id_option(self):
        data = tool_use_trajectory()
        assert render_trajectory_cmd5(data, template_id="cmd5-strict") == render_cmd5(
            {**trajectory_to_render_options(data), "template_id": "cmd5-strict"}
        )

    def test_cmd4_uses_jinja(self):
        rendered = render_trajectory_cmd4(tool_use_trajectory())
        assert "<|START_ACTION|>" in rendered
        assert "Paris is in France." in rendered

    def test_images(self):
        seen = []

        def placeholder(img):
            seen.append(img)
            return "<|IMG_{width}x{height}|>".format(**img["metadata"])

        data = trajectory(
            turn("User", text("Look:"), image()),
            turn("Chatbot", reasoning("Searching."), tool_calls(("c", {"query": "x"}))),
            turn("Tool", tool_results(("c", [dict(IMAGE), [dict(IMAGE)]]))),
            preamble=structured_preamble(tools=[SEARCH_TOOL]),
        )
        rendered = render_trajectory_cmd5(data, image_placeholder=placeholder)
        assert "<|START_TEXT|>Look:<|END_TEXT|><|IMG_4x4|>" in rendered
        assert (
            '<cofl:tool_result_item index="0">{"image_content": "<|IMG_4x4|>"}'
            "</cofl:tool_result_item>"
            '<cofl:tool_result_item index="1">[{"image_content": "<|IMG_4x4|>"}]'
            "</cofl:tool_result_item>"
        ) in rendered
        assert len(seen) == 3
        assert seen[0]["image_url"] == IMAGE["image_url"]

    def test_fixed_image_placeholder(self):
        data = trajectory(turn("User", image()))
        rendered = render_trajectory_cmd5(data, image_placeholder="<|IMG|>")
        assert "<|USER_TOKEN|><|IMG|><|END_OF_TURN_TOKEN|>" in rendered

    def test_tool_result_dict_with_extra_keys_is_a_document(self):
        data = trajectory(
            turn("User", text("Hi")),
            turn("Chatbot", tool_calls(("c", {"query": "x"}))),
            turn("Tool", tool_results(("c", [{"image_url": "u", "caption": "c"}]))),
            preamble=structured_preamble(tools=[SEARCH_TOOL]),
        )
        tool_message = trajectory_to_render_options(data)["messages"][2]
        assert tool_message["content"] == [
            {"type": "document", "document": {"image_url": "u", "caption": "c"}}
        ]


class TestUnsupported:
    @pytest.mark.parametrize(
        ("data", "message"),
        [
            (
                trajectory(turn("User", image())),
                "turn 0 (user): the trajectory contains images; pass image_placeholder",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    preamble=structured_preamble(
                        default_preamble={"content": "You are a pirate."}
                    ),
                ),
                "default_preamble has no slot",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    preamble=structured_preamble(
                        default_preamble={"mode": "non_interactive"}
                    ),
                ),
                "default_preamble has no slot",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    preamble=structured_preamble(
                        current_timestamp="2026-01-01T00:00:00"
                    ),
                ),
                "current_timestamp has no slot",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    preamble=structured_preamble(
                        tools=[
                            {
                                "mcp_tool_host_unique_id": "00000000-0000-0000-0000-000000000000",
                                "tool_name": "search",
                            }
                        ]
                    ),
                ),
                "MCP tool specs cannot be rendered",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    turn("Chatbot", text("Let me check."), tool_calls(("c", {}))),
                    preamble=structured_preamble(tools=[SEARCH_TOOL]),
                ),
                "turn 1 (chatbot): the templates drop text and images",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    turn("Chatbot", tool_calls(("a", {})), tool_calls(("b", {}))),
                    preamble=structured_preamble(tools=[SEARCH_TOOL]),
                ),
                "only one tool_calls content",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    turn("Chatbot", text("Hello"), reasoning("hmm")),
                    preamble=structured_preamble(),
                ),
                "reasoning must be the first content",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    turn("Chatbot", tool_calls(("c", {}))),
                    turn("Tool", tool_results(("c", [{"a": 1}]))),
                    turn("Chatbot", text("Hello", [span(0, 5, ("c", 0))]), text("!")),
                    preamble=structured_preamble(tools=[SEARCH_TOOL]),
                ),
                "grounding spans need the turn's text",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    turn("Chatbot", text("Hello", [span(0, 5, ("nope", 0))])),
                    preamble=structured_preamble(tools=[SEARCH_TOOL]),
                ),
                "unknown tool_call_id 'nope'",
            ),
            (
                trajectory(turn("User", text("Hi", [span(0, 2, ("c", 0))]))),
                "grounding spans are only rendered in chatbot turns",
            ),
            (
                trajectory(
                    turn("User", text("Hi")),
                    turn("Chatbot", tool_calls(("c", {}))),
                    turn("Tool", tool_results(("c", [[{"a": 1}]]))),
                    preamble=structured_preamble(tools=[SEARCH_TOOL]),
                ),
                "only render images inside a list tool result",
            ),
        ],
    )
    def test_raises(self, data, message):
        with pytest.raises(UnsupportedTrajectoryError, match=re.escape(message)):
            trajectory_to_render_options(data)

    def test_is_a_value_error(self):
        assert issubclass(UnsupportedTrajectoryError, ValueError)
