"""Render datatools ``AgentTrajectory`` data with the Melody Jinja templates.

``AgentTrajectory`` is the post-training trajectory format defined in
``datatools.types.agent_trajectory``. This module converts it into the options
accepted by ``render_cmd4`` / ``render_cmd5``, the same way the serving stack
converts API requests into Melody messages, so a trajectory renders exactly as
the API would render the equivalent request.

The converter works on the trajectory's JSON form and does not depend on
datatools. Pass either the pydantic model itself (anything with
``model_dump``) or the dict returned by ``trajectory.model_dump(mode="json")``.

Example::

    from cohere_melody.agent_trajectory import render_trajectory_cmd5

    prompt = render_trajectory_cmd5(
        trajectory.as_without_trailing_chatbot_turn(),
        image_placeholder="<|IMG_PATCH|>",
    )

Melody always appends the generation prompt (an open chatbot turn), so drop a
trailing chatbot turn first when you want an inference prompt.

What the Melody templates cannot express raises ``UnsupportedTrajectoryError``
instead of being dropped silently, with two exceptions that match the API:

* ``ToolResult.is_error`` is not rendered; every tool result renders as
  successful.
* ``StructuredPreamble.safety_preamble`` is ignored, as in all C4+ rendering.
"""

from __future__ import annotations

import json
from collections.abc import Callable, Mapping, Sequence
from typing import Any, Union

from cohere_melody._cohere_melody import render_cmd4, render_cmd5

__all__ = [
    "ImagePlaceholder",
    "UnsupportedTrajectoryError",
    "render_trajectory_cmd4",
    "render_trajectory_cmd5",
    "trajectory_to_render_options",
]

ImagePlaceholder = Union[str, Callable[[Mapping[str, Any]], str]]
"""Text rendered in place of an image.

Either a fixed string, or a callable that receives the image content dict
(``image_url``, ``mime_type``, ``metadata``, ...) and returns the string, for
example the image tokens for that image's size.
"""

_ROLES = {"System": "system", "User": "user", "Chatbot": "chatbot", "Tool": "tool"}
_IMAGE_FIELDS = frozenset({"image_url", "mime_type", "language_id", "metadata"})


class UnsupportedTrajectoryError(ValueError):
    """The trajectory uses a feature the Melody templates cannot render."""


def trajectory_to_render_options(
    trajectory: Any,
    *,
    image_placeholder: ImagePlaceholder | None = None,
) -> dict[str, Any]:
    """Convert an ``AgentTrajectory`` into ``render_cmd4`` / ``render_cmd5`` options.

    The result sets ``messages``, ``available_tools``, ``reasoning_type`` and
    ``grounding``, plus ``dev_instruction`` and ``platform_instruction`` when the
    preamble has them. Merge in any other render option (``template_id``,
    ``escaped_special_tokens``, ...) before rendering.

    Preamble mapping:

    * ``StructuredPreamble.platform_preamble`` -> ``platform_instruction``
    * ``StructuredPreamble.developer_preamble`` -> ``dev_instruction``
    * ``UnstructuredPreamble.content`` -> ``dev_instruction``
    * ``react_config.tools`` -> ``available_tools``
    * ``react_config.reasoning_effort`` -> ``reasoning_type`` (disabled
      without a ``react_config``)
    * ``react_config.grounding_style`` -> ``grounding`` (disabled without a
      ``react_config``). Grounding spans become citations only when it is
      ``"SPAN"``.

    Args:
        trajectory: An ``AgentTrajectory`` model, or its JSON-mode dict.
        image_placeholder: Text to render for each image. Required when the
            trajectory contains images.

    Raises:
        UnsupportedTrajectoryError: The trajectory uses a feature the
            templates cannot render, e.g. a custom default preamble, an MCP
            tool spec, or text alongside tool calls in one chatbot turn.
    """
    data = _as_json_dict(trajectory)
    preamble = data.get("preamble")
    react_config = None
    if preamble is not None and "content" not in preamble:
        react_config = preamble.get("react_config")

    options: dict[str, Any] = _preamble_options(preamble)
    grounding_enabled = False
    reasoning_enabled = False
    tools: list[dict[str, Any]] = []
    if react_config is not None:
        grounding_enabled = react_config["grounding_style"] == "SPAN"
        reasoning_enabled = react_config["reasoning_effort"] == "ON"
        tools = [_tool_spec(tool) for tool in react_config["tools"]]

    converter = _TurnConverter(image_placeholder, grounding_enabled)
    options["messages"] = [
        message
        for index, turn in enumerate(data["turns"])
        for message in converter.convert(turn, index)
    ]
    options["available_tools"] = tools
    options["reasoning_type"] = "enabled" if reasoning_enabled else "disabled"
    options["grounding"] = "enabled" if grounding_enabled else "disabled"
    return options


def render_trajectory_cmd4(
    trajectory: Any,
    *,
    image_placeholder: ImagePlaceholder | None = None,
    **options: Any,
) -> str:
    """Render an ``AgentTrajectory`` with a Command 4 Jinja template.

    ``options`` are extra ``render_cmd4`` options, e.g. ``template_id="cmd4-v2"``.
    They take precedence over the options derived from the trajectory.
    """
    config = trajectory_to_render_options(
        trajectory, image_placeholder=image_placeholder
    )
    config["use_jinja"] = True
    config.update(options)
    return render_cmd4(config)


def render_trajectory_cmd5(
    trajectory: Any,
    *,
    image_placeholder: ImagePlaceholder | None = None,
    **options: Any,
) -> str:
    """Render an ``AgentTrajectory`` with a Command 5 Jinja template.

    ``options`` are extra ``render_cmd5`` options, e.g.
    ``template_id="cmd5-strict"``. They take precedence over the options
    derived from the trajectory.
    """
    config = trajectory_to_render_options(
        trajectory, image_placeholder=image_placeholder
    )
    config.update(options)
    return render_cmd5(config)


def _as_json_dict(trajectory: Any) -> Mapping[str, Any]:
    if isinstance(trajectory, Mapping):
        return trajectory
    model_dump = getattr(trajectory, "model_dump", None)
    if callable(model_dump):
        return model_dump(mode="json")
    raise TypeError(
        "expected an AgentTrajectory or its JSON dict, got "
        f"{type(trajectory).__name__}"
    )


def _preamble_options(preamble: Mapping[str, Any] | None) -> dict[str, Any]:
    if preamble is None:
        return {}
    if "content" in preamble:
        return {"dev_instruction": preamble["content"]}

    default_preamble = preamble.get("default_preamble")
    if default_preamble is not None and (
        "content" in default_preamble
        or default_preamble.get("mode") == "non_interactive"
    ):
        raise UnsupportedTrajectoryError(
            "default_preamble has no slot in the Melody templates; move its "
            "content into platform_preamble or developer_preamble"
        )
    if preamble.get("current_timestamp") is not None:
        raise UnsupportedTrajectoryError(
            "current_timestamp has no slot in the Melody templates; move it "
            "into platform_preamble or developer_preamble"
        )

    options: dict[str, Any] = {}
    if preamble.get("platform_preamble") is not None:
        options["platform_instruction"] = preamble["platform_preamble"]["content"]
    if preamble.get("developer_preamble") is not None:
        options["dev_instruction"] = preamble["developer_preamble"]["content"]
    return options


def _tool_spec(tool: Mapping[str, Any]) -> dict[str, Any]:
    if "name" not in tool:
        raise UnsupportedTrajectoryError(
            "MCP tool specs cannot be rendered; convert them with "
            "MCPToolSpec.to_static_tool_spec() first"
        )
    return {
        "name": tool["name"],
        "description": tool["description"],
        "parameters": tool["parameters"],
    }


def _is_image(item: Mapping[str, Any]) -> bool:
    # A generic tool result shaped like an image becomes ImageToolResult when
    # AgentTrajectory parses its own JSON, so treat it the same way here.
    return isinstance(item.get("image_url"), str) and item.keys() <= _IMAGE_FIELDS


class _TurnConverter:
    """Converts turns in order, numbering tool calls like Melody does."""

    def __init__(
        self, image_placeholder: ImagePlaceholder | None, grounding_enabled: bool
    ) -> None:
        self._image_placeholder = image_placeholder
        self._grounding_enabled = grounding_enabled
        self._tool_call_indices: dict[str, int] = {}

    def convert(self, turn: Mapping[str, Any], index: int) -> list[dict[str, Any]]:
        role = _ROLES[turn["role"]]
        try:
            if role == "tool":
                return self._tool_messages(turn["contents"])
            if role == "chatbot":
                return [self._chatbot_message(turn["contents"])]
            return [self._input_message(role, turn["contents"])]
        except UnsupportedTrajectoryError as e:
            raise UnsupportedTrajectoryError(f"turn {index} ({role}): {e}") from None

    def _tool_call_index(self, tool_call_id: str) -> int:
        return self._tool_call_indices.setdefault(
            tool_call_id, len(self._tool_call_indices)
        )

    def _image(self, image: Mapping[str, Any]) -> dict[str, Any]:
        if self._image_placeholder is None:
            raise UnsupportedTrajectoryError(
                "the trajectory contains images; pass image_placeholder"
            )
        if callable(self._image_placeholder):
            placeholder = self._image_placeholder(image)
        else:
            placeholder = self._image_placeholder
        return {"type": "image", "image": {"template_placeholder": placeholder}}

    def _input_message(
        self, role: str, contents: Sequence[Mapping[str, Any]]
    ) -> dict[str, Any]:
        content = []
        for c in contents:
            if c["content_type"] == "image":
                content.append(self._image(c))
                continue
            if c.get("grounding_spans"):
                raise UnsupportedTrajectoryError(
                    "grounding spans are only rendered in chatbot turns"
                )
            content.append({"type": "text", "text": c["text"]})
        return {"role": role, "content": content}

    def _chatbot_message(self, contents: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
        content: list[dict[str, Any]] = []
        tool_calls: list[dict[str, Any]] = []
        citations: list[dict[str, Any]] = []
        for position, c in enumerate(contents):
            content_type = c["content_type"]
            if content_type == "tool_calls":
                if tool_calls:
                    raise UnsupportedTrajectoryError(
                        "a chatbot turn can have only one tool_calls content"
                    )
                tool_calls = [self._tool_call(tc) for tc in c["tool_calls"]]
            elif content_type == "reasoning":
                if position != 0:
                    raise UnsupportedTrajectoryError(
                        "reasoning must be the first content of a chatbot turn"
                    )
                citations += self._citations(c, is_thinking=True)
                content.append({"type": "thinking", "thinking": c["text"]})
            elif content_type == "text":
                citations += self._citations(c, is_thinking=False)
                content.append({"type": "text", "text": c["text"]})
            else:
                content.append(self._image(c))

        has_thinking = bool(content) and content[0]["type"] == "thinking"
        if tool_calls and len(content) > int(has_thinking):
            raise UnsupportedTrajectoryError(
                "the templates drop text and images from chatbot turns with "
                "tool calls"
            )
        # Melody puts non-thinking citations on the second content when the
        # first is thinking, or on the only content otherwise.
        if any(not c["is_thinking"] for c in citations) and len(content) != 1:
            if not has_thinking or len(content) != 2:
                raise UnsupportedTrajectoryError(
                    "grounding spans need the turn's text to be its only "
                    "non-reasoning content"
                )
        return {
            "role": "chatbot",
            "content": content,
            "tool_calls": tool_calls,
            "citations": citations,
        }

    def _tool_call(self, tool_call: Mapping[str, Any]) -> dict[str, Any]:
        self._tool_call_index(tool_call["id"])
        return {
            "id": tool_call["id"],
            "name": tool_call["name"],
            "parameters": json.dumps(tool_call["parameters"], ensure_ascii=False),
        }

    def _citations(
        self, text_content: Mapping[str, Any], is_thinking: bool
    ) -> list[dict[str, Any]]:
        spans = text_content.get("grounding_spans")
        if not spans or not self._grounding_enabled:
            return []
        text = text_content["text"]
        citations = []
        for span in spans:
            by_tool_call: dict[int, list[int]] = {}
            for result_id in span["tool_result_ids"]:
                tool_call_id = result_id["tool_call_id"]
                if tool_call_id not in self._tool_call_indices:
                    raise UnsupportedTrajectoryError(
                        f"grounding span cites unknown tool_call_id {tool_call_id!r}"
                    )
                by_tool_call.setdefault(
                    self._tool_call_indices[tool_call_id], []
                ).append(result_id.get("index", 0))
            citations.append(
                {
                    "start_index": span["start"],
                    "end_index": span["end"],
                    "text": text[span["start"] : span["end"]],
                    "is_thinking": is_thinking,
                    "sources": [
                        {
                            "tool_call_index": tool_call_index,
                            "tool_result_indices": sorted(indices),
                        }
                        for tool_call_index, indices in sorted(by_tool_call.items())
                    ],
                }
            )
        return citations

    def _tool_messages(
        self, contents: Sequence[Mapping[str, Any]]
    ) -> list[dict[str, Any]]:
        messages = []
        for c in contents:
            for tool_result in c["tool_results"]:
                tool_call_id = tool_result["tool_call_id"]
                self._tool_call_index(tool_call_id)
                messages.append(
                    {
                        "role": "tool",
                        "tool_call_id": tool_call_id,
                        "content": [
                            self._tool_result_item(item)
                            for item in tool_result["results"]
                        ],
                    }
                )
        return messages

    def _tool_result_item(self, item: Any) -> dict[str, Any]:
        if isinstance(item, Mapping):
            if _is_image(item):
                return self._image(item)
            return {"type": "document", "document": dict(item)}
        parts = []
        for part in item:
            if not _is_image(part):
                raise UnsupportedTrajectoryError(
                    "the templates can only render images inside a list tool " "result"
                )
            parts.append(self._image(part))
        return {"type": "multipart", "multipart": parts}
