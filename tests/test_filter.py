import pytest
from cohere_melody import PyFilter, PyFilterOptions

def test_simple_filter():
    f = PyFilter(PyFilterOptions().handle_multi_hop_cmd3().stream_tool_actions())
    fo = f.write_decoded("<|START_THINKING|>This is a")
    assert fo[0].text == "This is a"
    assert fo[0].is_tools_reason == True

    fo = f.write_decoded(" plan.<|END_THINKING|>")
    assert fo[0].text == " plan."
    assert fo[0].is_tools_reason == True

    fo = f.write_decoded("<|START_RESPONSE|>This is the final response.")
    assert fo[0].text == "This is the final response."
    assert fo[0].is_tools_reason == False
