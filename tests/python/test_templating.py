"""Tests for the templating functionality (render_cmd3, render_cmd4)."""

import pytest
from cohere_melody import render_cmd3, render_cmd4


class TestRenderCmd3:
    """Tests for the render_cmd3 function."""

    def test_empty_messages(self):
        """Test rendering with empty messages."""
        result = render_cmd3({"messages": []})
        assert isinstance(result, str)

    def test_single_user_message(self):
        """Test rendering with a single user message."""
        result = render_cmd3(
            {
                "messages": [
                    {"role": "user", "content": [{"type": "text", "text": "Hello!"}]}
                ]
            }
        )
        assert "USER" in result
        assert "Hello!" in result

    def test_user_and_assistant_messages(self):
        """Test rendering with user and assistant messages."""
        result = render_cmd3(
            {
                "messages": [
                    {"role": "user", "content": [{"type": "text", "text": "Hi"}]},
                    {
                        "role": "chatbot",
                        "content": [{"type": "text", "text": "Hello!"}],
                    },
                ]
            }
        )
        assert "USER" in result
        assert "CHATBOT" in result

    def test_with_dev_instruction(self):
        """Test rendering with developer instruction."""
        result = render_cmd3(
            {
                "messages": [],
                "dev_instruction": "You are a helpful assistant.",
            }
        )
        assert "helpful assistant" in result

    def test_with_tools(self):
        """Test rendering with available tools."""
        result = render_cmd3(
            {
                "messages": [],
                "available_tools": [
                    {
                        "name": "search",
                        "description": "Search the web",
                        "parameters": {"type": "object", "properties": {}},
                    }
                ],
            }
        )
        assert "search" in result

    def test_with_documents(self):
        """Test rendering with documents."""
        result = render_cmd3(
            {
                "messages": [],
                "documents": [{"title": "Doc 1", "content": "Some content"}],
            }
        )
        assert "Doc 1" in result or "content" in result

    @pytest.mark.parametrize(
        "safety_mode",
        ["none", "strict", "contextual"],
    )
    def test_safety_modes(self, safety_mode):
        """Test rendering with different safety modes."""
        result = render_cmd3(
            {
                "messages": [],
                "safety_mode": safety_mode,
            }
        )
        assert isinstance(result, str)

    @pytest.mark.parametrize(
        "citation_quality",
        ["off", "on"],
    )
    def test_citation_quality(self, citation_quality):
        """Test rendering with different citation quality settings."""
        result = render_cmd3(
            {
                "messages": [],
                "citation_quality": citation_quality,
            }
        )
        assert isinstance(result, str)

    def test_with_json_mode(self):
        """Test rendering with JSON mode enabled."""
        result = render_cmd3(
            {
                "messages": [],
                "json_mode": True,
            }
        )
        assert isinstance(result, str)

    def test_with_json_schema(self):
        """Test rendering with JSON schema."""
        result = render_cmd3(
            {
                "messages": [],
                "json_schema": '{"type": "object"}',
            }
        )
        assert isinstance(result, str)

    def test_with_response_prefix(self):
        """Test rendering with response prefix."""
        result = render_cmd3(
            {
                "messages": [],
                "response_prefix": "Sure, here is",
            }
        )
        assert isinstance(result, str)

    def test_invalid_config_type_raises_type_error(self):
        """Test that passing non-dict raises TypeError."""
        with pytest.raises(TypeError):
            render_cmd3("not a dict")

    def test_unknown_field_raises_value_error(self):
        """Test that unknown field raises ValueError."""
        with pytest.raises(ValueError):
            render_cmd3({"messages": [], "unknown_field": "value"})

    def test_invalid_role_raises_value_error(self):
        """Test that invalid role raises ValueError."""
        with pytest.raises(ValueError):
            render_cmd3(
                {
                    "messages": [
                        {
                            "role": "invalid_role",
                            "content": [{"type": "text", "text": "Hi"}],
                        }
                    ]
                }
            )

    def test_invalid_content_type_raises_value_error(self):
        """Test that invalid content type raises ValueError."""
        with pytest.raises(ValueError):
            render_cmd3(
                {
                    "messages": [
                        {
                            "role": "user",
                            "content": [{"type": "invalid_type", "text": "Hi"}],
                        }
                    ]
                }
            )


class TestRenderCmd4:
    """Tests for the render_cmd4 function."""

    def test_empty_messages(self):
        """Test rendering with empty messages."""
        result = render_cmd4({"messages": []})
        assert isinstance(result, str)

    def test_single_user_message(self):
        """Test rendering with a single user message."""
        result = render_cmd4(
            {
                "messages": [
                    {"role": "user", "content": [{"type": "text", "text": "Hello!"}]}
                ]
            }
        )
        assert "USER" in result
        assert "Hello!" in result

    def test_user_and_assistant_messages(self):
        """Test rendering with user and assistant messages."""
        result = render_cmd4(
            {
                "messages": [
                    {"role": "user", "content": [{"type": "text", "text": "Hi"}]},
                    {
                        "role": "chatbot",
                        "content": [{"type": "text", "text": "Hello!"}],
                    },
                ]
            }
        )
        assert "USER" in result
        assert "CHATBOT" in result

    def test_with_dev_instruction(self):
        """Test rendering with developer instruction."""
        result = render_cmd4(
            {
                "messages": [],
                "dev_instruction": "You are a helpful assistant.",
            }
        )
        assert isinstance(result, str)

    def test_with_platform_instruction(self):
        """Test rendering with platform instruction."""
        result = render_cmd4(
            {
                "messages": [],
                "platform_instruction": "Custom platform instruction",
            }
        )
        assert isinstance(result, str)

    def test_with_tools(self):
        """Test rendering with available tools."""
        result = render_cmd4(
            {
                "messages": [],
                "available_tools": [
                    {
                        "name": "calculator",
                        "description": "Perform calculations",
                        "parameters": {"type": "object", "properties": {}},
                    }
                ],
            }
        )
        assert "calculator" in result

    def test_with_documents(self):
        """Test rendering with documents."""
        result = render_cmd4(
            {
                "messages": [],
                "documents": [{"title": "Document", "text": "Content here"}],
            }
        )
        assert isinstance(result, str)

    @pytest.mark.parametrize("grounding", ["enabled", "disabled"])
    def test_grounding_options(self, grounding):
        """Test rendering with different grounding options."""
        result = render_cmd4({"messages": [], "grounding": grounding})
        assert isinstance(result, str)

    def test_with_json_mode(self):
        """Test rendering with JSON mode enabled."""
        result = render_cmd4(
            {
                "messages": [],
                "json_mode": True,
            }
        )
        assert isinstance(result, str)

    def test_with_json_schema(self):
        """Test rendering with JSON schema."""
        result = render_cmd4(
            {
                "messages": [],
                "json_schema": '{"type": "object"}',
            }
        )
        assert isinstance(result, str)

    def test_with_response_prefix(self):
        """Test rendering with response prefix."""
        result = render_cmd4(
            {
                "messages": [],
                "response_prefix": "Here is my response:",
            }
        )
        assert isinstance(result, str)

    def test_invalid_config_type_raises_type_error(self):
        """Test that passing non-dict raises TypeError."""
        with pytest.raises(TypeError):
            render_cmd4("not a dict")

    def test_unknown_field_raises_value_error(self):
        """Test that unknown field raises ValueError."""
        with pytest.raises(ValueError):
            render_cmd4({"messages": [], "unknown_field": "value"})

    def test_invalid_role_raises_value_error(self):
        """Test that invalid role raises ValueError."""
        with pytest.raises(ValueError):
            render_cmd4(
                {
                    "messages": [
                        {
                            "role": "invalid_role",
                            "content": [{"type": "text", "text": "Hi"}],
                        }
                    ]
                }
            )

    def test_invalid_grounding_raises_value_error(self):
        """Test that invalid grounding value raises ValueError."""
        with pytest.raises(ValueError):
            render_cmd4({"messages": [], "grounding": "invalid"})
