import json
import os
from enum import Enum
from os.path import basename, dirname, isdir, join
from typing import Any

import jinja2
import minijinja
import pytest


class Engine(Enum):
    JINJA2 = 1
    MINIJINJA = 2


def render_template_jinja2(template_dir: str, template_name: str, **kwargs: Any) -> str:
    # This jinja environment should match the huggingface settings from here:
    # https://github.com/huggingface/transformers/blob/57278c904c5158999d31a0db8bfcd63360c37b48/src/transformers/utils/chat_template_utils.py#L455-L460
    env = jinja2.Environment(
        loader=jinja2.FileSystemLoader(template_dir),
        lstrip_blocks=True,
        trim_blocks=True,
        extensions=["jinja2.ext.loopcontrols"],
    )
    if "cmd3-v1" in template_name:
        kwargs["add_generation_prompt"] = True
    # Overriding tojson with ensure_ascii=False so that tojson doesn't write unicode
    # characters as \uxxxx
    env.filters["tojson"] = lambda *args, **kwargs: json.dumps(
        *args, ensure_ascii=False, sort_keys=False, **kwargs
    )
    env.policies["json.dumps_kwargs"] = {"sort_keys": False}
    template = env.get_template(template_name)

    return template.render(**kwargs)


def render_template_minijinja(
    template_dir: str, template_name: str, **kwargs: Any
) -> str:
    def loader(name: str) -> str:
        content, _, _ = jinja2.FileSystemLoader(template_dir).get_source(
            None,  # type: ignore[arg-type] # Not used by this loader implementation
            name,
        )
        return content

    def tojson(*args: Any, **kwargs: Any) -> str:
        return json.dumps(*args, ensure_ascii=False, sort_keys=False, **kwargs)

    filters = {"tojson": tojson}

    # This jinja environment should match the huggingface settings from here:
    # https://github.com/huggingface/transformers/blob/57278c904c5158999d31a0db8bfcd63360c37b48/src/transformers/utils/chat_template_utils.py#L455-L460
    env = minijinja.Environment(
        loader=loader,
        filters=filters,
        lstrip_blocks=True,
        trim_blocks=True,
    )
    if "cmd3-v1" in template_name:
        kwargs["add_generation_prompt"] = True

    return env.render_template(template_name, **kwargs)


def render_template(
    template_dir: str,
    template_name: str,
    engine: Engine,
    **kwargs: Any,
) -> str:
    if engine is Engine.MINIJINJA:
        return render_template_minijinja(template_dir, template_name, **kwargs)

    return render_template_jinja2(template_dir, template_name, **kwargs)


# read json from file_path
def read_test_data(file_path: str) -> Any:
    with open(file_path) as file:
        return json.load(file)


def get_template_info(template_path: str) -> tuple[str, str]:
    # Get template directory, and the directory's name
    template_dir = dirname(template_path)

    # Get template name
    template_name = basename(template_path)

    return template_dir, template_name


@pytest.mark.parametrize(
    "template_path, test_dir",
    [
        (
            "templates/jinja/cmd3-v1.jinja",
            "jinja_tests/cmd3_v1_hf/chat_merged_template_v1",
        ),
        (
            "templates/jinja/cmd3-v2.jinja",
            "jinja_tests/cmd3_reasoning_hf/chat_merged_template",
        ),
        ("templates/jinja/cmd4-v1.jinja", "jinja_tests/cmd4_v1/chat_template"),
    ],
)
@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
def test_render_template(template_path: str, test_dir: str, engine: Engine) -> None:
    template_dir, template_name = get_template_info(template_path)

    # get all .json files from the test_dir
    test_files = [
        f
        for f in os.listdir(test_dir)
        # check not isdir instead of isfile so invalid symlinks are caught
        if not isdir(join(test_dir, f)) and f.endswith(".json")
    ]

    print(f"Running {len(test_files)} tests for {template_path}")
    errors = []
    for test_file in test_files:
        test_data = read_test_data(f"{test_dir}/{test_file}")
        rendered = render_template(template_dir, template_name, engine, **test_data)
        with open(f"{test_dir}/{test_file.replace('.json', '.txt')}") as f:
            expected = f.read()
        try:
            assert rendered == expected
        except AssertionError as e:
            print(f"Failed for template: {test_file}")
            e.add_note(f"input: {json.dumps(test_data, indent=2)}")
            e.add_note(f"test_file: {test_file}")
            errors.append(e)
    if errors:
        raise ExceptionGroup("Tests failed", errors)
