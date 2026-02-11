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
        outputs = cmd3_filter.write_decoded("Hello world")
        # Plain text may be buffered or pass through
        assert isinstance(outputs, list)
        assert outputs[0].text == "Hello world"
        assert outputs[0].is_reasoning is False

    def test_thinking_tags(self, cmd3_filter):
        """Test write_decoded with thinking tags."""
        outputs = cmd3_filter.write_decoded("<|START_THINKING|>This is a thought")
        assert len(outputs) > 0
        assert outputs[0].text == "This is a thought"
        assert outputs[0].is_reasoning is True

    def test_response_tags(self, cmd3_filter):
        """Test write_decoded with response tags."""
        outputs = cmd3_filter.write_decoded("<|START_RESPONSE|>Hello")
        assert len(outputs) > 0
        assert outputs[0].text == "Hello"
        assert outputs[0].is_reasoning is False

    def test_transition_thinking_to_response(self, cmd3_filter):
        """Test transitioning from thinking to response."""
        thinking_outputs = cmd3_filter.write_decoded("<|START_THINKING|>Thinking...")
        assert all(o.is_reasoning for o in thinking_outputs)

        outputs = cmd3_filter.write_decoded(
            "<|END_THINKING|><|START_RESPONSE|>Response"
        )
        # Should have transitioned to response mode with non-reasoning output
        response_outputs = [o for o in outputs if not o.is_reasoning]
        assert len(response_outputs) > 0
        assert "Response" in "".join(o.text for o in response_outputs)


class TestPyFilterFlushPartials:
    """Tests for PyFilter.flush_partials method."""

    def test_flush_with_buffered_content(self):
        """Test flush_partials returns buffered content when chunk_size > 1."""
        f = PyFilter.cmd3(chunk_size=10)
        # Write less than chunk_size characters
        outputs = f.write_decoded("<|START_RESPONSE|>Hi")
        # Should be buffered
        assert len(outputs) == 0 or "".join(o.text for o in outputs) == ""
        # flush_partials should return buffered content
        outputs = f.flush_partials()
        text = "".join(o.text for o in outputs)
        assert "Hi" in text

    def test_flush_empty_filter(self):
        """Test flush_partials on fresh filter returns empty list."""
        f = PyFilter.cmd3()
        outputs = f.flush_partials()
        assert outputs == []

    def test_flush_after_complete_output(self):
        """Test flush_partials after content already emitted."""
        f = PyFilter.cmd3()
        f.write_decoded("<|START_RESPONSE|>Hello")
        outputs = f.flush_partials()
        assert isinstance(outputs, list)


class TestPyFilterStreamingWorkflow:
    """Tests for complete streaming workflows."""

    def test_complete_workflow(self):
        """Test a complete streaming workflow."""
        f = PyFilter.cmd3()
        all_text = []

        tokens = ["<|START_RESPONSE|>", "Hello", " ", "world", "!"]
        for token in tokens:
            for o in f.write_decoded(token):
                all_text.append(o.text)

        for o in f.flush_partials():
            all_text.append(o.text)

        result = "".join(all_text)
        assert "Hello world!" in result

    def test_thinking_then_response_workflow(self):
        """Test workflow with thinking followed by response."""
        f = PyFilter.cmd3()

        # Thinking phase
        outputs = f.write_decoded("<|START_THINKING|>This is a")
        assert outputs[0].text == "This is a"
        assert outputs[0].is_reasoning is True

        outputs = f.write_decoded(" plan.<|END_THINKING|>")
        assert outputs[0].text == " plan."
        assert outputs[0].is_reasoning is True

        # Response phase
        outputs = f.write_decoded("<|START_RESPONSE|>This is the final response.")
        assert outputs[0].text == "This is the final response."
        assert outputs[0].is_reasoning is False


class TestFilterOutput:
    """Tests for FilterOutput attributes."""

    @pytest.fixture
    def sample_output(self):
        """Get a sample FilterOutput from a filter."""
        f = PyFilter.cmd3()
        outputs = f.write_decoded("<|START_RESPONSE|>Test")
        outputs.extend(f.flush_partials())
        return outputs[0] if outputs else None

    def test_has_text_attribute(self, sample_output):
        """Test that FilterOutput has text attribute."""
        if sample_output:
            assert hasattr(sample_output, "text")
            assert isinstance(sample_output.text, str)

    def test_has_is_reasoning_attribute(self, sample_output):
        """Test that FilterOutput has is_reasoning attribute."""
        if sample_output:
            assert hasattr(sample_output, "is_reasoning")
            assert isinstance(sample_output.is_reasoning, bool)

    def test_thinking_output_is_reasoning_true(self):
        """Test that thinking output has is_reasoning=True."""
        f = PyFilter.cmd3()
        outputs = f.write_decoded("<|START_THINKING|>Thought")
        assert len(outputs) > 0
        assert outputs[0].is_reasoning is True

    def test_response_output_is_reasoning_false(self):
        """Test that response output has is_reasoning=False."""
        f = PyFilter.cmd3()
        outputs = f.write_decoded("<|START_RESPONSE|>Response")
        assert len(outputs) > 0
        assert outputs[0].is_reasoning is False


class TestPyFilterOptions:
    """Tests for PyFilterOptions builder pattern."""

    def test_cmd3_option(self):
        """Test cmd3() builder method."""
        opts = PyFilterOptions().cmd3()
        f = PyFilter(opts)
        outputs = f.write_decoded("<|START_RESPONSE|>Hello")
        assert len(outputs) > 0
        assert outputs[0].text == "Hello"

    def test_cmd4_option(self):
        """Test cmd4() builder method."""
        opts = PyFilterOptions().cmd4()
        f = PyFilter(opts)
        outputs = f.write_decoded("Hello")
        assert len(outputs) > 0
        assert outputs[0].text == "Hello"

    def test_rag_option(self):
        """Test rag() builder method."""
        opts = PyFilterOptions().rag()
        f = PyFilter(opts)
        outputs = f.write_decoded("Hello")
        assert isinstance(outputs, list)

    def test_multi_hop_option(self):
        """Test multi_hop() builder method."""
        opts = PyFilterOptions().multi_hop()
        f = PyFilter(opts)
        outputs = f.write_decoded("Hello")
        assert isinstance(outputs, list)

    def test_search_query_option(self):
        """Test search_query() builder method."""
        opts = PyFilterOptions().search_query()
        f = PyFilter(opts)
        outputs = f.write_decoded("Hello")
        assert isinstance(outputs, list)

    def test_with_chunk_size(self):
        """Test with_chunk_size() builder method."""
        opts = PyFilterOptions().cmd3().with_chunk_size(10)
        f = PyFilter(opts)
        # With chunk_size=10, small outputs should be buffered
        outputs = f.write_decoded("<|START_RESPONSE|>Hi")
        assert len(outputs) == 0
        # Flush should return buffered content
        outputs = f.flush_partials()
        text = "".join(o.text for o in outputs)
        assert "Hi" in text

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
        # Original options should be unchanged
        assert opts1 is not opts2
        assert opts2 is not opts3
