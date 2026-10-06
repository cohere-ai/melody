"""Render cmd5 like OpenAI-compatible servers (vLLM, transformers); compare to Melody.

Each tests/templating/jinja/cmd5/<case>/ with an openai_input.json holds the
OpenAI-shaped equivalent of the Melody input.json: raw tool strings, tool results
serialized as JSON in text content, and tool call arguments parsed into dicts (as
vLLM does before rendering). The rendered prompt must match the native Melody
output.txt byte for byte.
"""

import os
from os.path import isfile, join

import pytest

from test_jinja import Engine, read_test_data, render_template

CMD5_CASES_DIR = "../cmd5"
CMD5_TEMPLATE_DIR = "templates/jinja"
CMD5_TEMPLATE_NAME = "cmd5.jinja"

CASES = sorted(
    case
    for case in os.listdir(CMD5_CASES_DIR)
    if isfile(join(CMD5_CASES_DIR, case, "openai_input.json"))
)


def test_cases_found() -> None:
    assert CASES, f"no openai_input.json fixtures were found in {CMD5_CASES_DIR}"


@pytest.mark.parametrize("case", CASES)
@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
def test_openai_input_matches_native(case: str, engine: Engine) -> None:
    test_data = read_test_data(join(CMD5_CASES_DIR, case, "openai_input.json"))
    rendered = render_template(
        CMD5_TEMPLATE_DIR, CMD5_TEMPLATE_NAME, engine, **test_data
    )
    with open(join(CMD5_CASES_DIR, case, "output.txt")) as f:
        expected = f.read()
    assert rendered == expected
