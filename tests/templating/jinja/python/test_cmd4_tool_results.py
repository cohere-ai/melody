import pytest

from test_jinja import Engine, render_template

CMD4_TEMPLATE_DIR = "templates/jinja"


@pytest.mark.parametrize("engine", [Engine.JINJA2, Engine.MINIJINJA])
@pytest.mark.parametrize(
    "template_name", ["cmd4-v1.jinja", "cmd4-v2.jinja", "cmd4-hf.jinja"]
)
def test_tool_result_images_render_placeholders(
    engine: Engine, template_name: str
) -> None:
    rendered = render_template(
        CMD4_TEMPLATE_DIR,
        template_name,
        engine,
        messages=[
            {
                "role": "tool",
                "tool_call_id": "0",
                "content": [
                    {"type": "image"},
                    {
                        "type": "image",
                        "image": {
                            "template_placeholder": (
                                "<|START_IMAGE|>imagedata<|END_IMAGE|>"
                            )
                        },
                    },
                ],
            },
        ],
        add_generation_prompt=True,
        bos_token="<BOS_TOKEN>",
        regen_tool_call_ids=False,
    )
    placeholders = [
        "<|IMG_PATCH|>",
        "<|START_IMAGE|>imagedata<|END_IMAGE|>",
    ]
    for index, placeholder in enumerate(placeholders):
        assert f'"{index}": {{"image_content": "{placeholder}"}}' in rendered
