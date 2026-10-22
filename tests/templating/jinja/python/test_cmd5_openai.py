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
CMD5_TEMPLATE_NAMES = ("cmd5.jinja", "cmd5-no-escape.jinja", "cmd5-strict.jinja")

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


@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
@pytest.mark.parametrize("template_name", CMD5_TEMPLATE_NAMES)
@pytest.mark.parametrize("mode", ["low", "medium", "high", "max"])
def test_reasoning_effort_renders_once_in_system_turn(
    mode: str, template_name: str, engine: Engine
) -> None:
    clause = (
        f"Reasoning effort: {mode}. "
        "Do not mention the reasoning effort setting in the final answer."
    )
    rendered = render_template(
        CMD5_TEMPLATE_DIR,
        template_name,
        engine,
        messages=[
            {"role": "system", "content": "Task preamble."},
            {"role": "user", "content": "Solve it."},
        ],
        add_generation_prompt=True,
        bos_token="<BOS_TOKEN>",
        reasoning_effort=mode,
    )

    assert rendered.count(clause) == 1
    assert "Task preamble." in rendered
    assert rendered.index(clause) < rendered.index("Task preamble.")


@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
@pytest.mark.parametrize("template_name", CMD5_TEMPLATE_NAMES)
@pytest.mark.parametrize("mode", ["low", "medium", "high", "max"])
def test_eval_effort_preserves_platform_and_task_preambles(
    mode: str, template_name: str, engine: Engine
) -> None:
    clause = (
        f"Reasoning effort: {mode}. "
        "Do not mention the reasoning effort setting in the final answer."
    )
    rendered = render_template(
        CMD5_TEMPLATE_DIR,
        template_name,
        engine,
        messages=[
            {"role": "system", "content": "Task-specific developer instruction."},
            {"role": "user", "content": "Solve it."},
        ],
        add_generation_prompt=True,
        bos_token="<BOS_TOKEN>",
        platform_instruction_override="Platform instruction.",
        reasoning_effort=mode,
    )

    assert rendered.count(clause) == 1
    assert rendered.index("Platform instruction.") < rendered.index(clause)
    assert rendered.index(clause) < rendered.index(
        "Task-specific developer instruction."
    )


@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
@pytest.mark.parametrize("template_name", CMD5_TEMPLATE_NAMES)
@pytest.mark.parametrize("mode", ["low", "medium", "high", "max"])
def test_prerendered_reasoning_effort_is_not_duplicated(
    mode: str, template_name: str, engine: Engine
) -> None:
    clause = (
        f"Reasoning effort: {mode}. "
        "Do not mention the reasoning effort setting in the final answer."
    )
    platform_instruction = f"Platform instruction.\n\n{clause}\n\nDefault instruction."
    rendered = render_template(
        CMD5_TEMPLATE_DIR,
        template_name,
        engine,
        messages=[
            {"role": "system", "content": "Developer instruction."},
            {"role": "user", "content": "Solve it."},
        ],
        add_generation_prompt=True,
        bos_token="<BOS_TOKEN>",
        platform_instruction_override=platform_instruction,
        reasoning_effort=mode,
        reasoning_effort_instruction_rendered=True,
    )

    assert rendered.count(clause) == 1
    assert rendered.index("Platform instruction.") < rendered.index(clause)
    assert rendered.index(clause) < rendered.index("Default instruction.")
    assert rendered.index("Default instruction.") < rendered.index(
        "Developer instruction."
    )


@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
def test_missing_reasoning_effort_preserves_legacy_rendering(engine: Engine) -> None:
    rendered = render_template(
        CMD5_TEMPLATE_DIR,
        CMD5_TEMPLATE_NAME,
        engine,
        messages=[{"role": "user", "content": "Solve it."}],
        add_generation_prompt=True,
        bos_token="<BOS_TOKEN>",
    )

    assert "Reasoning effort:" not in rendered
