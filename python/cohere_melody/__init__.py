"""Templating rendering and generation parsing for Cohere models."""

from typing import TYPE_CHECKING

from cohere_melody._cohere_melody import (
    AccumulatedToolCall,
    FilterAggregatedResult,
    PyFilter,
    PyFilterOptions,
    SearchQueryDelta,
    parse_truncated_vision_generation,
    parse_vision_generation,
    render_cmd3,
    render_cmd4,
    render_cmd5,
)
from cohere_melody.templating import (
    Citation,
    CitationSource,
    Content,
    Document,
    Image,
    Message,
    RenderCmd3Options,
    RenderCmd4Options,
    RenderCmd5Options,
    Tool,
    ToolCall,
)

if TYPE_CHECKING:
    # Only constructed by the parser, so the native module does not export them.
    from cohere_melody._cohere_melody import FilterCitation as FilterCitation
    from cohere_melody._cohere_melody import Source as Source

__all__ = [
    "AccumulatedToolCall",
    "Citation",
    "CitationSource",
    "Content",
    "Document",
    "FilterAggregatedResult",
    "Image",
    "Message",
    "PyFilter",
    "PyFilterOptions",
    "RenderCmd3Options",
    "RenderCmd4Options",
    "RenderCmd5Options",
    "SearchQueryDelta",
    "Tool",
    "ToolCall",
    "parse_truncated_vision_generation",
    "parse_vision_generation",
    "render_cmd3",
    "render_cmd4",
    "render_cmd5",
]
