"""vLLM integration for *melody*.

Wraps the melody functionality into vLLM parsers for reasoning and tool calls.
"""

from importlib.metadata import version as _get_version
from typing import TYPE_CHECKING, Optional, Sequence, Union

from packaging.version import Version as _Version

from vllm.reasoning import ReasoningParser, ReasoningParserManager
from vllm.tool_parsers import ToolParser, ToolParserManager
from vllm.transformers_utils.tokenizer import AnyTokenizer

# vllm > 0.14.1 reorganized OpenAI entrypoint imports (https://github.com/vllm-project/vllm/pull/32240)
_VLLM_POST_0_14_1 = _Version(_get_version("vllm")) > _Version("0.14.1")
if _VLLM_POST_0_14_1:
    from vllm.entrypoints.openai.chat_completion.protocol import (
        ChatCompletionRequest,
    )
    from vllm.entrypoints.openai.engine.protocol import (
        DeltaFunctionCall,
        DeltaMessage,
        DeltaToolCall,
        ExtractedToolCallInformation,
        FunctionCall,
        ToolCall,
    )
    from vllm.entrypoints.openai.responses.protocol import (
        ResponsesRequest,
    )
else:
    from vllm.entrypoints.openai.protocol import (
        ChatCompletionRequest,
        DeltaFunctionCall,
        DeltaMessage,
        DeltaToolCall,
        ExtractedToolCallInformation,
        FunctionCall,
        ResponsesRequest,
        ToolCall,
    )


try:
    from cohere_melody import PyFilter, PyFilterOptions

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

    def uses_only_delta_streaming(self) -> bool:
        return True

    def extract_reasoning(
        self, model_output: str, request: ChatCompletionRequest | ResponsesRequest
    ) -> tuple[Optional[str], Optional[str]]:
        melody = PyFilter(
            PyFilterOptions()
            .cmd3()
            .remove_token("<|START_ACTION|>")
            .remove_token("<|END_ACTION|>")
        )
        result = melody.process_full_text(model_output)
        return result.reasoning, result.content

    def extract_content_ids(self, input_ids: list[int]) -> list[int]:
        melody = PyFilter(
            PyFilterOptions()
            .cmd3()
            .remove_token("<|START_ACTION|>")
            .remove_token("<|END_ACTION|>")
        )
        token_buf = []
        decoded_chunks = []
        chunk_token_ids = []
        content_ids = []
        for t in input_ids:
            token_buf.append(t)
            token_str = self.model_tokenizer.decode(
                token_buf, skip_special_tokens=False
            )
            # buffer tokens that generate incomplete strings
            if token_str.endswith(REPLACEMENT_CHAR):
                continue

            decoded_chunks.append(token_str)
            chunk_token_ids.append(token_buf)
            token_buf = []

        content_mask = melody.classify_content_chunks(decoded_chunks)
        for has_content, token_ids in zip(
            content_mask,
            chunk_token_ids,
            strict=False,
        ):
            if has_content:
                content_ids.extend(token_ids)
        return content_ids

    def is_reasoning_end(self, input_ids: Sequence[int]) -> bool:
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

    def uses_only_delta_streaming(self) -> bool:
        return True

    def extract_tool_calls(
        self,
        model_output: str,
        request: ChatCompletionRequest,
    ) -> ExtractedToolCallInformation:
        result = self.melody.process_full_text(model_output)

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
