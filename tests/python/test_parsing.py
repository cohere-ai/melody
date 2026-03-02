"""Tests for the parsing functionality (PyFilter)."""

import pytest
from cohere_melody import PyFilter, PyFilterOptions


class TestPyFilterWriteDecoded:
    """Tests for PyFilter.write_decoded method."""

    @pytest.fixture
    def cmd3_filter(self):
        """Create a fresh cmd3 filter for each test."""
        return PyFilter.cmd3()

    def test_plain_text(self, cmd3_filter):
        """Test write_decoded with plain text."""
        result = cmd3_filter.write_decoded("Hello world")
        assert result.content == "Hello world"
        assert result.reasoning is None

    def test_thinking_tags(self, cmd3_filter):
        """Test write_decoded with thinking tags."""
        result = cmd3_filter.write_decoded("<|START_THINKING|>This is a thought")
        assert result.reasoning == "This is a thought"
        assert result.content is None

    def test_response_tags(self, cmd3_filter):
        """Test write_decoded with response tags."""
        result = cmd3_filter.write_decoded("<|START_RESPONSE|>Hello")
        assert result.content == "Hello"
        assert result.reasoning is None

    def test_transition_thinking_to_response(self, cmd3_filter):
        """Test transitioning from thinking to response."""
        result = cmd3_filter.write_decoded("<|START_THINKING|>Thinking...")
        assert result.reasoning is not None

        result = cmd3_filter.write_decoded("<|END_THINKING|><|START_RESPONSE|>Response")
        assert result.content is not None
        assert "Response" in result.content


class TestPyFilterFlushPartials:
    """Tests for PyFilter.flush_partials method."""

    def test_flush_with_buffered_content(self):
        """Test flush_partials returns buffered content when chunk_size > 1."""
        f = PyFilter.cmd3(chunk_size=10)
        result = f.write_decoded("<|START_RESPONSE|>Hi")
        assert result.content is None or result.content == ""
        result = f.flush_partials()
        assert result.content is not None
        assert "Hi" in result.content

    def test_flush_empty_filter(self):
        """Test flush_partials on fresh filter returns default result."""
        f = PyFilter.cmd3()
        result = f.flush_partials()
        assert result.content is None
        assert result.reasoning is None

    def test_flush_after_complete_output(self):
        """Test flush_partials after content already emitted."""
        f = PyFilter.cmd3()
        f.write_decoded("<|START_RESPONSE|>Hello")
        result = f.flush_partials()
        assert result is not None


class TestPyFilterStreamingWorkflow:
    """Tests for complete streaming workflows."""

    def test_complete_workflow(self):
        """Test a complete streaming workflow."""
        f = PyFilter.cmd3()
        all_text = []

        tokens = ["<|START_RESPONSE|>", "Hello", " ", "world", "!"]
        for token in tokens:
            result = f.write_decoded(token)
            if result.content is not None:
                all_text.append(result.content)

        result = f.flush_partials()
        if result.content is not None:
            all_text.append(result.content)

        full_text = "".join(all_text)
        assert "Hello world!" in full_text

    def test_thinking_then_response_workflow(self):
        """Test workflow with thinking followed by response."""
        f = PyFilter.cmd3()

        result = f.write_decoded("<|START_THINKING|>This is a")
        assert result.reasoning == "This is a"
        assert result.content is None

        result = f.write_decoded(" plan.<|END_THINKING|>")
        assert result.reasoning == " plan."

        result = f.write_decoded("<|START_RESPONSE|>This is the final response.")
        assert result.content == "This is the final response."
        assert result.reasoning is None


class TestAggregatedResult:
    """Tests for AggregatedResult attributes."""

    @pytest.fixture
    def sample_result(self):
        """Get a sample AggregatedResult from a filter."""
        f = PyFilter.cmd3()
        result = f.write_decoded("<|START_RESPONSE|>Test")
        return result

    def test_has_content_attribute(self, sample_result):
        """Test that AggregatedResult has content attribute."""
        assert hasattr(sample_result, "content")

    def test_has_reasoning_attribute(self, sample_result):
        """Test that AggregatedResult has reasoning attribute."""
        assert hasattr(sample_result, "reasoning")

    def test_thinking_output_has_reasoning(self):
        """Test that thinking output has reasoning set."""
        f = PyFilter.cmd3()
        result = f.write_decoded("<|START_THINKING|>Thought")
        assert result.reasoning is not None
        assert result.content is None

    def test_response_output_has_content(self):
        """Test that response output has content set."""
        f = PyFilter.cmd3()
        result = f.write_decoded("<|START_RESPONSE|>Response")
        assert result.content is not None
        assert result.reasoning is None


class TestPyFilterOptions:
    """Tests for PyFilterOptions builder pattern."""

    def test_cmd3_option(self):
        """Test cmd3() builder method."""
        opts = PyFilterOptions().cmd3()
        f = PyFilter(opts)
        result = f.write_decoded("<|START_RESPONSE|>Hello")
        assert result.content == "Hello"

    def test_cmd4_option(self):
        """Test cmd4() builder method."""
        opts = PyFilterOptions().cmd4()
        f = PyFilter(opts)
        result = f.write_decoded("Hello")
        assert result.content == "Hello"

    def test_rag_option(self):
        """Test rag() builder method."""
        opts = PyFilterOptions().rag()
        f = PyFilter(opts)
        result = f.write_decoded("Hello")
        assert result is not None

    def test_multi_hop_option(self):
        """Test multi_hop() builder method."""
        opts = PyFilterOptions().multi_hop()
        f = PyFilter(opts)
        result = f.write_decoded("Hello")
        assert result is not None

    def test_search_query_option(self):
        """Test search_query() builder method."""
        opts = PyFilterOptions().search_query()
        f = PyFilter(opts)
        result = f.write_decoded("Hello")
        assert result is not None

    def test_with_chunk_size(self):
        """Test with_chunk_size() builder method."""
        opts = PyFilterOptions().cmd3().with_chunk_size(10)
        f = PyFilter(opts)
        result = f.write_decoded("<|START_RESPONSE|>Hi")
        assert result.content is None or result.content == ""
        result = f.flush_partials()
        assert result.content is not None
        assert "Hi" in result.content

    def test_chained_options(self):
        """Test chaining multiple builder methods."""
        opts = (
            PyFilterOptions()
            .cmd3()
            .with_chunk_size(5)
            .with_left_trimmed()
            .with_right_trimmed()
        )
        f = PyFilter(opts)
        assert f is not None

    def test_with_inclusive_stops(self):
        """Test with_inclusive_stops() builder method."""
        opts = PyFilterOptions().cmd3().with_inclusive_stops(["STOP"])
        f = PyFilter(opts)
        assert f is not None

    def test_with_exclusive_stops(self):
        """Test with_exclusive_stops() builder method."""
        opts = PyFilterOptions().cmd3().with_exclusive_stops(["END"])
        f = PyFilter(opts)
        assert f is not None

    def test_stream_tool_actions(self):
        """Test stream_tool_actions() builder method."""
        opts = PyFilterOptions().cmd3().stream_tool_actions()
        f = PyFilter(opts)
        assert f is not None

    def test_stream_processed_params(self):
        """Test stream_processed_params() builder method."""
        opts = PyFilterOptions().cmd3().stream_processed_params()
        f = PyFilter(opts)
        assert f is not None

    def test_stream_non_grounded_answer(self):
        """Test stream_non_grounded_answer() builder method."""
        opts = PyFilterOptions().cmd3().stream_non_grounded_answer()
        f = PyFilter(opts)
        assert f is not None

    def test_remove_token(self):
        """Test remove_token() builder method."""
        opts = PyFilterOptions().cmd3().remove_token("<|START_THINKING|>")
        f = PyFilter(opts)
        assert f is not None

    def test_options_are_immutable(self):
        """Test that builder methods return new instances."""
        opts1 = PyFilterOptions()
        opts2 = opts1.cmd3()
        opts3 = opts2.with_chunk_size(10)
        assert opts1 is not opts2
        assert opts2 is not opts3
