import json
import os
from os.path import isfile, join
from typing import Any

import pytest

from test_jinja import Engine, get_template_info, read_test_data, render_template


def get_all_files(test_dir: str) -> list[str]:
    return [
        f
        for f in os.listdir(test_dir)
        if isfile(join(test_dir, f)) and f.endswith(".json")
    ]


def get_test_tuples(
    template_to_test_files: dict[str, list[str]], test_dir: str
) -> list[tuple[str, str, str]]:
    template_test_files: list[tuple[str, str, str]] = []
    for template, test_files in template_to_test_files.items():
        nt = len(test_files)
        template_test_files.extend(zip([template] * nt, [test_dir] * nt, test_files))
    return template_test_files


def get_tests(template: str, test_dir: str) -> list[tuple[str, str, str]]:
    test_files = get_all_files(test_dir)
    template_to_test_files = {template: test_files}
    return get_test_tuples(template_to_test_files, test_dir)


def get_cmd3_v2_tests() -> list[tuple[str, str, str]]:
    # get all .json files from the test_dir
    template_to_test_files = {}
    test_dir = "liquid_tests/rag/cmd3-v2"
    all_test_files = get_all_files(test_dir)

    merged_template = "templates/jinja/cmd3-v2.jinja"
    template_to_test_files[merged_template] = all_test_files

    return get_test_tuples(template_to_test_files, test_dir)


def get_cmd3_v1_tests() -> list[tuple[str, str, str]]:
    # get all .json files from the test_dir
    merged_template = "templates/jinja/cmd3-v1.jinja"

    chat_test_dir = "liquid_tests/chat/cmd3-v1"
    rag_test_dir = "liquid_tests/rag/cmd3-v1"

    return get_tests(merged_template, chat_test_dir) + get_tests(
        merged_template, rag_test_dir
    )


def get_cmd3_v3_tests() -> list[tuple[str, str, str]]:
    # get all .json files from the test_dir
    merged_template = "templates/jinja/cmd3-v3.jinja"

    test_dir = "liquid_tests/rag/cmd3-v3"

    return get_tests(merged_template, test_dir)


def get_cmd4_v1_tests() -> list[tuple[str, str, str]]:
    # get all .json files from the test_dir
    chat_template = "templates/jinja/cmd4-v1.jinja"
    rag_test_dir = "liquid_tests/rag/cmd4-v1"
    return get_tests(chat_template, rag_test_dir)


def get_template_test_files() -> list[tuple[str, str, str]]:
    cmd3_v2_tests = get_cmd3_v2_tests()
    cmd3_v1_tests = get_cmd3_v1_tests()
    cmd3_v3_tests = get_cmd3_v3_tests()
    cmd4_v1_tests = get_cmd4_v1_tests()
    return [*cmd3_v2_tests, *cmd3_v1_tests, *cmd3_v3_tests, *cmd4_v1_tests]


template_test_files = get_template_test_files()


# This test is to run liquid template tests against the jinja template
@pytest.mark.parametrize("template_path, test_dir, test_files", template_test_files)
@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
def test_render_template(  # noqa: C901
    template_path: str,
    test_dir: str,
    test_files: str | list[str],
    engine: Engine,
) -> None:
    template_dir, template_name = get_template_info(template_path)

    if isinstance(test_files, str):
        test_files = [test_files]
    errors = []
    for test_file in test_files:
        print("Running test: ", test_file)
        test_data = read_test_data(f"{test_dir}/{test_file}")
        test_data = test_data["variables"]

        test_data["bos_token"] = "<BOS_TOKEN>"
        test_data["add_generation_prompt"] = True

        test_data["enable_citations"] = True
        test_data["regen_tool_call_ids"] = False
        if test_data.get("citation_mode") == "OFF":
            test_data["enable_citations"] = False
        if test_data.get("skip_thinking") is None:
            test_data["skip_thinking"] = False
        test_data["reasoning"] = test_data.get("reasoning_options", {}).get(
            "enabled", False
        )
        if test_data.get("json_mode"):
            test_data["response_format"] = {"type": "json_object"}
            json_schema = test_data.get("json_schema")
            if json_schema:
                test_data["response_format"]["schema"] = json_schema
        avail_tools = test_data.get("available_tools")
        if avail_tools:
            tools = []
            for avail_tool in avail_tools:
                defn = avail_tool.get(
                    "definition", {"description": "", "json_schema": "{}"}
                )
                tool: dict[str, Any] = {"type": "function"}
                tool["function"] = {
                    "name": avail_tool.get("name", ""),
                    "description": defn["description"],
                    "parameters": json.loads(defn["json_schema"]),
                }
                tools.append(tool)
            test_data["tools"] = tools
        role_map = {
            "User": "user",
            "Chatbot": "assistant",
            "Tool": "tool",
        }
        docs = test_data.get("documents", [])
        for idx in range(len(docs)):
            docs[idx] = json.loads(docs[idx])

        messages = test_data["messages"]
        preamble = test_data.get("preamble")
        if preamble:
            messages.insert(0, {"role": "system", "content": preamble})
        new_msgs = []
        for message in messages:
            message["role"] = role_map.get(message["role"], message["role"])
            if "content" in message:
                if isinstance(message["content"], str):
                    continue
                for idx, content in enumerate(message["content"]):
                    if (
                        content["type"] == "thinking"
                        and content.get("thinking") is None
                    ):
                        content["thinking"] = content.get("data", "")
                    if content["type"] == "text" and content.get("text") is None:
                        if idx == 0 and "tool_calls" in message:
                            content["type"] = "thinking"
                            content["thinking"] = content.get("data", "")
                        else:
                            content["text"] = content.get("data", "")
            try:
                if "tool_calls" in message:
                    msg_tool_calls = message["tool_calls"]
                    for idx in range(len(msg_tool_calls)):
                        otcall = json.loads(msg_tool_calls[idx])
                        tcall = {
                            "id": otcall["tool_call_id"],
                            "type": "function",
                            "function": {
                                "name": otcall["tool_name"],
                                "arguments": otcall["parameters"],
                            },
                        }
                        msg_tool_calls[idx] = tcall
                if "tool_results" in message:
                    if "content" not in message:
                        message["content"] = []
                    tool_call_id_to_msg = {}
                    if message["tool_results"]:
                        res_call_id = str(message["tool_results"][0]["tool_call_id"])
                        tool_call_id_to_msg[res_call_id] = message
                        message["tool_call_id"] = res_call_id
                    for res in message["tool_results"]:
                        res_call_id = str(res["tool_call_id"])
                        if res_call_id not in tool_call_id_to_msg:
                            # We need to add a new message for this tool result
                            new_msg = {
                                "role": "tool",
                                "tool_call_id": res_call_id,
                                "content": [],
                            }
                            new_msgs.append((new_msg, message))
                            tool_call_id_to_msg[res_call_id] = new_msg
                        cur_msg = tool_call_id_to_msg[res_call_id]
                        for doc in res["documents"]:
                            cur_msg["content"].append(json.loads(doc))

            except TypeError:
                print(f"Failed for {test_file}")
                raise
        for msg, orig_msg in new_msgs[::-1]:
            orig_msg_idx = messages.index(orig_msg)
            messages.insert(orig_msg_idx + 1, msg)

        # print(json.dumps(test_data, indent=2))
        rendered = render_template(template_dir, template_name, engine, **test_data)
        with open(f"{test_dir}/{test_file.replace('.json', '.txt')}") as f:
            expected = f.read()
        # work around for strange difference where liquid logic is inserting space
        # around brackets
        # Ticket to fix in linear: https://linear.app/cohereai/issue/PTS-8620/fix-liquid-template-spacing
        expected = expected.replace('{ "', '{"').replace('" }', '"}')
        try:
            assert rendered == expected
        except AssertionError as e:
            print(f"Failed for template: {test_file}")
            e.add_note(f"input: {json.dumps(test_data, indent=2)}")
            e.add_note(f"test_file: {test_file}")
            errors.append(e)
    if errors:
        raise ExceptionGroup("Tests failed", errors)
