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
        assert result.reasoning == "Hello"

    def test_cmd5_option(self):
        """Test cmd5() builder method parses cofl tool calls."""
        opts = PyFilterOptions().cmd5()
        f = PyFilter(opts)
        text = (
            "<|START_THINKING|>I should search.<|END_THINKING|>"
            '<cofl:tool_calls><cofl:tool_call id="call_0" name="web_search">'
            '<cofl:tool_param name="query" string="true">test</cofl:tool_param>'
            "</cofl:tool_call></cofl:tool_calls>"
        )
        result = f.process_full_text(text)
        assert result.reasoning == "I should search."
        assert len(result.tool_calls) == 1
        assert result.tool_calls[0].id == "call_0"
        assert result.tool_calls[0].name == "web_search"
        assert result.tool_calls[0].arguments == '{"query": "test"}'

    def test_cmd5_static_factory(self):
        """Test PyFilter.cmd5() factory parses cofl tool calls."""
        f = PyFilter.cmd5()
        text = (
            '<cofl:tool_calls><cofl:tool_call id="0" name="search">'
            '<cofl:tool_param name="q" string="true">hello</cofl:tool_param>'
            "</cofl:tool_call></cofl:tool_calls>"
        )
        result = f.process_full_text(text)
        assert len(result.tool_calls) == 1
        assert result.tool_calls[0].arguments == '{"q": "hello"}'

    def test_cmd4_does_not_parse_cofl_tool_calls(self):
        """cmd4 filter must not treat cofl tags as structured tool calls."""
        f = PyFilter.cmd4()
        text = (
            '<cofl:tool_calls><cofl:tool_call id="0" name="search">'
            '<cofl:tool_param name="q" string="true">hello</cofl:tool_param>'
            "</cofl:tool_call></cofl:tool_calls>"
        )
        result = f.process_full_text(text)
        assert result.tool_calls == []

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

    def test_cofl_no_xml_text_decode(self):
        """Test cofl_no_xml_text_decode() parses unescaped cofl param bodies."""
        opts = PyFilterOptions().cmd5().cofl_no_xml_text_decode()
        f = PyFilter(opts)
        text = (
            "<|START_THINKING|>think<|END_THINKING|>"
            '<cofl:tool_calls><cofl:tool_call id="0" name="run&lt;cmd&gt;&amp;tool">'
            '<cofl:tool_param name="str_param" string="true">'
            'value with <tag> & "quotes"'
            "</cofl:tool_param></cofl:tool_call></cofl:tool_calls>"
        )
        result = f.process_full_text(text)
        assert len(result.tool_calls) == 1
        assert result.tool_calls[0].name == "run<cmd>&tool"
        import json

        parsed = json.loads(result.tool_calls[0].arguments)
        assert parsed["str_param"] == 'value with <tag> & "quotes"'

    def test_options_are_immutable(self):
        """Test that builder methods return new instances."""
        opts1 = PyFilterOptions()
        opts2 = opts1.cmd3()
        opts3 = opts2.with_chunk_size(10)
        assert opts1 is not opts2
        assert opts2 is not opts3


class TestWithMessageHistory:
    """End-to-end tests: parser resolves citation indices to document IDs."""

    def _cmd3_input(self, citation: str) -> str:
        return f"<|START_RESPONSE|>foo {citation}<|END_RESPONSE|>"

    def test_without_message_history_document_ids_empty(self):
        """When no lookup is configured, document_ids stays empty."""
        f = PyFilter(PyFilterOptions().cmd3())
        result = f.process_full_text(self._cmd3_input("<co>bar</co: 0:[0,1]>"))
        assert len(result.citations) == 1
        src = result.citations[0].sources[0]
        assert src.tool_call_index == 0
        assert src.tool_result_indices == [0, 1]
        assert src.document_ids == []

    def test_top_level_documents_resolve(self):
        """Top-level documents live in tool_call_index 0."""
        opts = (
            PyFilterOptions()
            .cmd3()
            .with_message_history(documents=[{"id": "doc-a"}, {"id": "doc-b"}])
        )
        f = PyFilter(opts)
        result = f.process_full_text(self._cmd3_input("<co>bar</co: 0:[0,1]>"))
        src = result.citations[0].sources[0]
        assert src.document_ids == ["doc-a", "doc-b"]

    def test_tool_results_resolve(self):
        """Tool result documents resolve via their assigned tool_call_index."""
        messages = [
            {
                "role": "chatbot",
                "content": [],
                "tool_calls": [
                    {"id": "call_1", "name": "search", "parameters": "{}"},
                ],
            },
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": [
                    {"type": "document", "document": {"id": "res-x"}},
                    {"type": "document", "document": {"id": "res-y"}},
                    {"type": "document", "document": {"id": "res-z"}},
                ],
            },
        ]
        opts = PyFilterOptions().cmd3().with_message_history(messages=messages)
        f = PyFilter(opts)
        result = f.process_full_text(self._cmd3_input("<co>bar</co: 0:[0,2]>"))
        src = result.citations[0].sources[0]
        assert src.document_ids == ["res-x", "res-z"]

    def test_docs_and_tool_calls_interleave_correctly(self):
        """Top-level docs sit at index 0; tool calls start at index 1."""
        documents = [{"id": "doc-a"}]
        messages = [
            {
                "role": "chatbot",
                "content": [],
                "tool_calls": [
                    {"id": "call_1", "name": "search", "parameters": "{}"},
                ],
            },
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": [
                    {"type": "document", "document": {"id": "res-1"}},
                ],
            },
        ]
        opts = (
            PyFilterOptions()
            .cmd3()
            .with_message_history(messages=messages, documents=documents)
        )
        f = PyFilter(opts)
        result = f.process_full_text(self._cmd3_input("<co>bar</co: 0:[0],1:[0]>"))
        cit = result.citations[0]
        assert len(cit.sources) == 2
        assert cit.sources[0].document_ids == ["doc-a"]
        assert cit.sources[1].document_ids == ["res-1"]

    def test_out_of_bounds_indices_handled_gracefully(self):
        """Out-of-bounds tool_result_indices produce empty strings."""
        opts = (
            PyFilterOptions().cmd3().with_message_history(documents=[{"id": "doc-a"}])
        )
        f = PyFilter(opts)
        result = f.process_full_text(self._cmd3_input("<co>bar</co: 0:[0,3]>"))
        src = result.citations[0].sources[0]
        assert src.document_ids == ["doc-a", ""]

    def test_arguments_default_to_empty(self):
        """Both messages and documents default to empty lists. With no history
        to build a lookup from, the parser leaves document_ids empty (same as
        never calling with_message_history at all)."""
        opts = PyFilterOptions().cmd3().with_message_history()
        f = PyFilter(opts)
        result = f.process_full_text(self._cmd3_input("<co>bar</co: 0:[0]>"))
        assert result.citations[0].sources[0].document_ids == []

    def test_positional_arguments_work(self):
        """messages and documents can be passed positionally, matching Rust."""
        opts = PyFilterOptions().cmd3().with_message_history([], [{"id": "doc-a"}])
        f = PyFilter(opts)
        result = f.process_full_text(self._cmd3_input("<co>bar</co: 0:[0]>"))
        assert result.citations[0].sources[0].document_ids == ["doc-a"]

    def test_template_shape_errors_do_not_raise(self):
        """Template-shape issues (duplicate tool_call_id here) are the
        renderer's concern, not the parser's. `with_message_history` accepts
        them silently and still produces a usable lookup."""
        messages = [
            {
                "role": "chatbot",
                "content": [],
                "tool_calls": [{"id": "call_1", "name": "search", "parameters": "{}"}],
            },
            {
                "role": "chatbot",
                "content": [],
                "tool_calls": [{"id": "call_1", "name": "search", "parameters": "{}"}],
            },
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": [{"type": "document", "document": {"id": "res-1"}}],
            },
        ]
        opts = PyFilterOptions().cmd3().with_message_history(messages=messages)
        f = PyFilter(opts)
        result = f.process_full_text(self._cmd3_input("<co>bar</co: 0:[0]>"))
        assert result.citations[0].sources[0].document_ids == ["res-1"]

    def test_non_list_messages_raises(self):
        """Argument-deserialisation failures still raise ValueError."""
        with pytest.raises(ValueError):
            PyFilterOptions().cmd3().with_message_history(messages="not a list")
