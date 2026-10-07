"""Render cmd5 like OpenAI-compatible servers (vLLM, transformers); compare to Melody.

Each openai_input*.json in tests/templating/jinja/cmd5/<case>/ holds an OpenAI-shaped
equivalent of the Melody input.json: raw tool strings, tool call arguments parsed into
dicts (as vLLM does before rendering), and tool results serialized as JSON Lines in
text content, which cmd5 reads unless json_tool_results is false. Variants cover the
shapes vLLM may hand to the template (one string, or one text part per result). The
rendered prompt must match the native Melody output.txt byte for byte.
"""

import os
from os.path import isdir, join

import pytest

from test_jinja import Engine, read_test_data, render_template

CMD5_CASES_DIR = "../cmd5"
CMD5_TEMPLATE_DIR = "templates/jinja"
CMD5_TEMPLATE_NAME = "cmd5.jinja"

CASES = sorted(
    (case, input_file)
    for case in os.listdir(CMD5_CASES_DIR)
    if isdir(join(CMD5_CASES_DIR, case))
    for input_file in os.listdir(join(CMD5_CASES_DIR, case))
    if input_file.startswith("openai_input") and input_file.endswith(".json")
)


def test_cases_found() -> None:
    assert CASES, f"no openai_input*.json fixtures were found in {CMD5_CASES_DIR}"


@pytest.mark.parametrize("case, input_file", CASES)
@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
def test_openai_input_matches_native(
    case: str, input_file: str, engine: Engine
) -> None:
    test_data = read_test_data(join(CMD5_CASES_DIR, case, input_file))
    rendered = render_template(
        CMD5_TEMPLATE_DIR, CMD5_TEMPLATE_NAME, engine, **test_data
    )
    with open(join(CMD5_CASES_DIR, case, "output.txt")) as f:
        expected = f.read()
    assert rendered == expected


@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
def test_json_tool_results_false_renders_text_as_plain_text(engine: Engine) -> None:
    rendered = render_template(
        CMD5_TEMPLATE_DIR,
        CMD5_TEMPLATE_NAME,
        engine,
        messages=[
            {"role": "user", "content": "Run it."},
            {
                "role": "assistant",
                "tool_calls": [
                    {
                        "id": "0",
                        "type": "function",
                        "function": {"name": "run_shell", "arguments": {}},
                    }
                ],
            },
            {"role": "tool", "tool_call_id": "0", "content": "first line\nsecond line"},
        ],
        add_generation_prompt=True,
        bos_token="<BOS_TOKEN>",
        json_tool_results=False,
    )
    assert (
        '<cofl:tool_result_item index="0">{"content": "first line\\nsecond line"}'
        "</cofl:tool_result_item></cofl:tool_result>" in rendered
    )
