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

if TYPE_CHECKING:
    # Only constructed by the parser, so the native module does not export them.
    from cohere_melody._cohere_melody import FilterCitation as FilterCitation
    from cohere_melody._cohere_melody import Source as Source

__all__ = [
    "AccumulatedToolCall",
    "FilterAggregatedResult",
    "PyFilter",
    "PyFilterOptions",
    "SearchQueryDelta",
    "parse_truncated_vision_generation",
    "parse_vision_generation",
    "render_cmd3",
    "render_cmd4",
    "render_cmd5",
]
