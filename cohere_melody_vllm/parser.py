"""vLLM integration for *melody*.

Wraps the melody functionality into vLLM parsers for reasoning and tool calls.
"""

from typing import Optional, Sequence, Union
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
from vllm.entrypoints.openai.tool_parsers import ToolParser, ToolParserManager
from vllm.transformers_utils.tokenizer import AnyTokenizer


try:
    from cohere_melody import PyFilter, PyFilterOptions  # type: ignore

except ModuleNotFoundError:
    raise RuntimeError("The compiled melody bindings are not available.")

REPLACEMENT_CHAR = "\ufffd"


@ReasoningParserManager.register_module(["cohere2"])
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

        out = self.melody.write_decoded(delta_text)

        content = None
        reasoning_content = None
        delta_tool_calls: list[DeltaToolCall] = []
        for o in out:
            if o.text is not None:
                if o.is_reasoning:
                    reasoning_content = (
                        "" if reasoning_content is None else reasoning_content
                    )
                    reasoning_content += o.text
                else:
                    content = "" if content is None else content
                    content += o.text
            if o.tool_call_delta is not None:
                delta_tool_call = DeltaToolCall(
                    id=o.tool_call_delta.id,
                    index=o.tool_call_delta.index,
                    type="function",
                    function=DeltaFunctionCall(
                        name=o.tool_call_delta.name,
                        arguments=o.tool_call_delta.raw_param_delta,
                    ),
                )
                delta_tool_calls.append(delta_tool_call)

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

            out = melody.write_decoded(token_str)
            for o in out:
                if o.text is not None:
                    if o.is_reasoning:
                        reasoning_content = (
                            "" if reasoning_content is None else reasoning_content
                        )
                        reasoning_content += o.text
                    else:
                        content = "" if content is None else content
                        content += o.text

            token_buf = []
        return reasoning_content, content


# TODO: implement other abstract methods if needed
#     @abstractmethod
#     def is_reasoning_end(self, input_ids: list[int]) -> bool:
#         """
#         Check if the reasoning content ends in the input_ids.

#         It is used in structured engines like `xgrammar` to check if the
#         reasoning content ends in the model output.

#         Parameters:
#         input_ids: list[int]
#             The input_ids of the model output.

#         Returns:
#         bool
#             True if the reasoning content ends in the input_ids.
#         """

#     @abstractmethod
#     def extract_content_ids(self, input_ids: list[int]) -> list[int]:
#         """
#         Extract content token ids from the input_ids.
#         Parameters:
#         input_ids: list[int]
#             The input_ids of the model output.
#         Returns:
#         list[int]
#             The extracted content from the input_ids.
#         """


@ToolParserManager.register_module(["cohere2"])
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

        out = self.melody.write_decoded(delta_text)

        delta_tool_calls = []
        for o in out:
            if o.tool_call_delta is not None:
                delta_tool_call = DeltaToolCall(
                    id=o.tool_call_delta.id,
                    index=o.tool_call_delta.index,
                    type="function",
                    function=DeltaFunctionCall(
                        name=o.tool_call_delta.name,
                        arguments=o.tool_call_delta.raw_param_delta,
                    ),
                )
                delta_tool_calls.append(delta_tool_call)

        if len(delta_tool_calls) > 0:
            return DeltaMessage(tool_calls=delta_tool_calls)

    def extract_tool_calls(
        self,
        model_output: str,
        request: ChatCompletionRequest,
    ) -> ExtractedToolCallInformation:
        tool_calls: list[ToolCall] = []
        content: str | None = None
        token_buf = []
        # tokenize to provide token size string fragments to melody
        for t in self.model_tokenizer.encode(model_output, add_special_tokens=False):
            token_buf.append(t)
            token_str = self.model_tokenizer.decode(
                token_buf, skip_special_tokens=False
            )
            # buffer tokens that generate incomplete strings
            if token_str.endswith(REPLACEMENT_CHAR):
                continue

            out = self.melody.write_decoded(token_str)
            for o in out:
                if o.text is not None:
                    content = "" if content is None else content
                    content += o.text
                if o.tool_call_delta is not None:
                    if o.tool_call_delta.id != "":
                        tool_calls.append(
                            ToolCall(
                                id=o.tool_call_delta.id,
                                type="function",
                                function=FunctionCall(name="", arguments=""),
                            )
                        )
                    if o.tool_call_delta.name != "":
                        tool_calls[o.tool_call_delta.index].function.name = (
                            o.tool_call_delta.name
                        )
                    tool_calls[
                        o.tool_call_delta.index
                    ].function.arguments += o.tool_call_delta.raw_param_delta

            token_buf = []

        return ExtractedToolCallInformation(
            tools_called=len(tool_calls) > 0,
            tool_calls=tool_calls,
            content=content,
        )
