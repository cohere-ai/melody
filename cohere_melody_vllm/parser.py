"""vLLM integration for *melody*.

Wraps the melody functionality into vLLM parsers for reasoning and tool calls.
"""

from typing import Optional, Sequence, Union, TYPE_CHECKING
from vllm.entrypoints.openai.protocol import (
    ChatCompletionRequest,
    ResponsesRequest,
    DeltaMessage,
    DeltaToolCall,
    DeltaFunctionCall,
    ExtractedToolCallInformation,
    FunctionCall,
    ToolCall,
)
from vllm.reasoning import ReasoningParser, ReasoningParserManager
from vllm.tool_parsers import ToolParser, ToolParserManager
from vllm.transformers_utils.tokenizer import AnyTokenizer

try:
    from cohere_melody import PyFilter, PyFilterOptions  # type: ignore

except ModuleNotFoundError:
    raise RuntimeError("The compiled melody bindings are not available.")

REPLACEMENT_CHAR = "\ufffd"


class CohereCommand2ReasoningParser(ReasoningParser):

    def __init__(self, tokenizer: AnyTokenizer, *args, **kwargs):
        super().__init__(tokenizer, *args, **kwargs)
        self.melody = PyFilter(PyFilterOptions().cmd3())

    def extract_reasoning_streaming(
        self,
        previous_text: str,
        current_text: str,
        delta_text: str,
        previous_token_ids: Sequence[int],
        current_token_ids: Sequence[int],
        delta_token_ids: Sequence[int],
    ) -> Union[DeltaMessage, None]:

        result = self.melody.write_decoded(delta_text)

        content = result.content
        reasoning_content = result.reasoning
        delta_tool_calls = [
            DeltaToolCall(
                id=tc.id,
                index=tc.index,
                type="function",
                function=DeltaFunctionCall(
                    name=tc.name,
                    arguments=tc.arguments,
                ),
            )
            for tc in result.tool_calls
        ]

        if content is None and reasoning_content is None and len(delta_tool_calls) == 0:
            return None

        msg = DeltaMessage()
        if content is not None:
            msg.content = content
        if reasoning_content is not None:
            msg.reasoning_content = reasoning_content
        if len(delta_tool_calls) > 0:
            msg.tool_calls = delta_tool_calls

        return msg

    def extract_reasoning(
        self, model_output: str, request: ChatCompletionRequest | ResponsesRequest
    ) -> tuple[Optional[str], Optional[str]]:
        reasoning_content = None
        content = None
        # create a new melody parser that ignores special tool action tokens
        # since the tool parser will be called on the resulting content
        melody = PyFilter(
            PyFilterOptions()
            .cmd3()
            .remove_token("<|START_ACTION|>")
            .remove_token("<|END_ACTION|>")
        )
        # tokenize to provide token size string fragments to melody
        tokens = self.model_tokenizer.encode(model_output, add_special_tokens=False)
        token_buf = []
        for t in tokens:
            token_buf.append(t)
            token_str = self.model_tokenizer.decode(
                token_buf, skip_special_tokens=False
            )
            # buffer tokens that generate incomplete strings
            if token_str.endswith(REPLACEMENT_CHAR):
                continue

            result = melody.write_decoded(token_str)
            if result.reasoning is not None:
                reasoning_content = (
                    "" if reasoning_content is None else reasoning_content
                )
                reasoning_content += result.reasoning
            if result.content is not None:
                content = "" if content is None else content
                content += result.content

            token_buf = []
        return reasoning_content, content

    def extract_content_ids(self, input_ids: list[int]) -> list[int]:
        melody = PyFilter(
            PyFilterOptions()
            .cmd3()
            .remove_token("<|START_ACTION|>")
            .remove_token("<|END_ACTION|>")
        )
        token_buf = []
        content_ids = []
        for t in input_ids:
            token_buf.append(t)
            token_str = self.model_tokenizer.decode(
                token_buf, skip_special_tokens=False
            )
            # buffer tokens that generate incomplete strings
            if token_str.endswith(REPLACEMENT_CHAR):
                continue

            result = melody.write_decoded(token_str)
            if result.content is not None:
                content_ids.extend(token_buf)

            token_buf = []
        return content_ids

    def is_reasoning_end(self, input_ids: list[int]) -> bool:
        end_token_id = self.model_tokenizer.convert_tokens_to_ids("<|END_THINKING|>")
        return any(input_id == end_token_id for input_id in reversed(input_ids))


if not TYPE_CHECKING:
    ReasoningParserManager.register_module(["cohere2"])(CohereCommand2ReasoningParser)


class CohereCommand2ToolParser(ToolParser):

    def __init__(self, tokenizer: AnyTokenizer):
        super().__init__(tokenizer)
        self.melody = PyFilter(PyFilterOptions().cmd3())

    def adjust_request(self, request: ChatCompletionRequest) -> ChatCompletionRequest:
        request = super().adjust_request(request)
        request.skip_special_tokens = False
        return request

    def extract_tool_calls_streaming(
        self,
        previous_text: str,
        current_text: str,
        delta_text: str,
        previous_token_ids: Sequence[int],
        current_token_ids: Sequence[int],
        delta_token_ids: Sequence[int],
        request: ChatCompletionRequest,
    ) -> Union[DeltaMessage, None]:

        result = self.melody.write_decoded(delta_text)

        if not result.tool_calls:
            return None

        delta_tool_calls = [
            DeltaToolCall(
                id=tc.id,
                index=tc.index,
                type="function",
                function=DeltaFunctionCall(
                    name=tc.name,
                    arguments=tc.arguments,
                ),
            )
            for tc in result.tool_calls
        ]

        return DeltaMessage(tool_calls=delta_tool_calls)

    def extract_tool_calls(
        self,
        model_output: str,
        request: ChatCompletionRequest,
    ) -> ExtractedToolCallInformation:
        tokens = self.model_tokenizer.encode(model_output, add_special_tokens=False)
        token_buf = []
        token_strings = []
        for t in tokens:
            token_buf.append(t)
            token_str = self.model_tokenizer.decode(
                token_buf, skip_special_tokens=False
            )
            if token_str.endswith(REPLACEMENT_CHAR):
                continue
            token_strings.append(token_str)
            token_buf = []

        result = self.melody.process_full(token_strings)

        tool_calls = [
            ToolCall(
                id=tc.id,
                type="function",
                function=FunctionCall(name=tc.name, arguments=tc.arguments),
            )
            for tc in result.tool_calls
        ]

        return ExtractedToolCallInformation(
            tools_called=len(tool_calls) > 0,
            tool_calls=tool_calls,
            content=result.content,
        )


if not TYPE_CHECKING:
    ToolParserManager.register_module(["cohere2"])(CohereCommand2ToolParser)
