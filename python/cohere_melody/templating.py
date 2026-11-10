"""Typed shapes of the configs accepted by ``render_cmd3``, ``render_cmd4`` and ``render_cmd5``.

The render functions take plain dicts, so these are ``TypedDict`` definitions:
they cost nothing at runtime, and a type checker or IDE uses them to check and
complete the keys you write::

    from cohere_melody import RenderCmd5Options, render_cmd5

    options: RenderCmd5Options = {
        "messages": [
            {"role": "user", "content": [{"type": "text", "text": "What is the capital of France?"}]},
        ],
        "dev_instruction": "Answer in one word.",
    }
    prompt = render_cmd5(options)

Every options key is optional; Melody fills in the defaults noted on each
field. Melody rejects keys it does not know, so a typo fails the render instead
of being ignored. Enum-like string values (``role``, ``type``, ``grounding``
and so on) are matched case-insensitively.

These mirror the Rust structs ``RenderCmd3Options`` and ``RenderCmd4Options`` in
``src/templating/lib.rs``, ``Message`` and its parts in
``src/templating/types.rs``, and ``FilterCitation`` and ``Source`` in
``src/parsing/types.rs``.
"""

from typing import Any, Literal, TypedDict

__all__ = [
    "Citation",
    "CitationQuality",
    "CitationSource",
    "Content",
    "ContentType",
    "Document",
    "Grounding",
    "Image",
    "Message",
    "ReasoningType",
    "RenderCmd3Options",
    "RenderCmd4Options",
    "RenderCmd5Options",
    "Role",
    "SafetyMode",
    "Tool",
    "ToolCall",
]

Role = Literal["system", "user", "chatbot", "tool"]
ContentType = Literal["text", "thinking", "image", "document", "multipart"]
Grounding = Literal["enabled", "disabled"]
ReasoningType = Literal["enabled", "disabled"]
SafetyMode = Literal["none", "strict", "contextual"]
CitationQuality = Literal["on", "off"]

Document = dict[str, Any]
"""A grounding document or tool result: any JSON object."""

# Required keys live in a base class and optional keys in a ``total=False``
# subclass, because ``typing.NotRequired`` needs Python 3.11 and the package
# supports 3.10 without depending on ``typing_extensions``.


class Image(TypedDict):
    template_placeholder: str
    """Text rendered in place of the image, such as the image token sequence."""


class _ContentRequired(TypedDict):
    type: ContentType


class Content(_ContentRequired, total=False):
    """One content block of a message. Set the key that matches ``type``."""

    text: str | None
    thinking: str | None
    image: Image | None
    document: Document | None
    multipart: "list[Content] | None"


class ToolCall(TypedDict):
    id: str
    name: str
    parameters: str
    """The call's arguments as a JSON-encoded string, not a dict."""


class _CitationSourceRequired(TypedDict):
    tool_call_index: int
    """Position of the cited tool call among all tool calls in ``messages``."""
    tool_result_indices: list[int]
    """Positions of the cited results within that tool call's tool message."""


class CitationSource(_CitationSourceRequired, total=False):
    document_ids: list[str]


class Citation(TypedDict):
    start_index: int
    """Character offset where the cited span starts in the message text."""
    end_index: int
    """Character offset where the cited span ends (exclusive)."""
    text: str
    sources: list[CitationSource]
    is_thinking: bool
    """True when the span is in the message's thinking rather than its text."""


class _MessageRequired(TypedDict):
    role: Role


class Message(_MessageRequired, total=False):
    content: list[Content]
    tool_calls: list[ToolCall]
    """Tool calls made by a ``chatbot`` message."""
    tool_call_id: str | None
    """For a ``tool`` message, the ``id`` of the tool call it answers."""
    citations: list[Citation]
    """Cited spans of a ``chatbot`` message."""


class Tool(TypedDict):
    name: str
    description: str
    parameters: dict[str, Any]
    """JSON schema of the tool's arguments."""


class RenderCmd3Options(TypedDict, total=False):
    """Config for ``render_cmd3``."""

    messages: list[Message]
    template_id: str | None
    """Built-in Jinja template, used with ``use_jinja``: ``"cmd3-v1"``, ``"cmd3-v2"`` or ``"cmd3-v3"``."""
    template: str
    """Liquid template source, used when ``use_jinja`` is false."""
    template_jinja: str
    """Jinja template source; an empty string means the default template."""
    use_jinja: bool
    """Render with Jinja instead of Liquid. Default: false."""
    dev_instruction: str | None
    documents: list[Document]
    available_tools: list[Tool]
    safety_mode: SafetyMode | None
    citation_quality: CitationQuality | None
    """Default: ``"on"``."""
    reasoning_type: ReasoningType | None
    skip_preamble: bool
    response_prefix: str | None
    json_schema: str | None
    """JSON schema for structured output, as a JSON-encoded string."""
    json_mode: bool
    additional_template_fields: dict[str, Any]
    """Extra variables passed to the template."""
    escaped_special_tokens: dict[str, str]
    """Replacements applied to message and document text, so it cannot contain special tokens
    such as ``"<|START_TEXT|>"``: each key is replaced by its value."""


class RenderCmd4Options(TypedDict, total=False):
    """Config for ``render_cmd4`` and ``render_cmd5``."""

    messages: list[Message]
    template_id: str | None
    """Built-in Jinja template. ``render_cmd4`` (with ``use_jinja``): ``"cmd4-v1"``,
    ``"cmd4-v2"`` or ``"cmd4-hf"``. ``render_cmd5``: ``"cmd5"``, ``"cmd5-no-escape"`` or
    ``"cmd5-strict"``."""
    template: str
    """Liquid template source, used by ``render_cmd4`` when ``use_jinja`` is false.
    ``render_cmd5`` only renders Jinja templates."""
    template_jinja: str
    """Jinja template source; an empty string means the default template."""
    use_jinja: bool
    """``render_cmd4`` only: render with Jinja instead of Liquid. Default: false."""
    dev_instruction: str | None
    platform_instruction: str | None
    """Replaces the default platform instruction."""
    documents: list[Document]
    available_tools: list[Tool]
    grounding: Grounding | None
    """Default: ``"enabled"``."""
    reasoning_type: ReasoningType | None
    response_prefix: str | None
    json_schema: str | None
    """JSON schema for structured output, as a JSON-encoded string."""
    json_mode: bool
    additional_template_fields: dict[str, Any]
    """Extra variables passed to the template."""
    escaped_special_tokens: dict[str, str]
    """Replacements applied to message and document text, so it cannot contain special tokens
    such as ``"<|START_TEXT|>"``: each key is replaced by its value."""


RenderCmd5Options = RenderCmd4Options
