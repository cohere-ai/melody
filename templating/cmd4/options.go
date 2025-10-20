package cmd4

import (
	"regexp"

	"github.com/cohere-ai/melody"
)

type options struct {
	template string

	devInstruction           *string
	platformInstruction      string
	documents                []melody.Document
	availableTools           []melody.Tool
	grounding                melody.Grounding
	responsePrefix           string
	escapedSpecialTokens     map[string]string
	jsonSchema               *string
	jsonMode                 bool
	additionalTemplateFields map[string]any
}
type Option func(*options) error

func WithTemplate(template string) Option {
	return func(c *options) error {
		c.template = template
		return nil
	}
}

func WithDeveloperInstruction(devInstruction *string) Option {
	return func(c *options) error {
		c.devInstruction = devInstruction
		return nil
	}
}

func WithPlatformInstruction(platformInstruction string) Option {
	return func(c *options) error {
		c.platformInstruction = platformInstruction
		return nil
	}
}

func WithDocuments(documents []melody.Document) Option {
	return func(c *options) error {
		c.documents = documents
		return nil
	}
}

func WithAvailableTools(availableTools []melody.Tool) Option {
	return func(c *options) error {
		c.availableTools = availableTools
		return nil
	}
}

func WithGrounding(grounding melody.Grounding) Option {
	return func(c *options) (err error) {
		c.grounding = grounding
		return err
	}
}

func WithResponsePrefix(responsePrefix string) Option {
	return func(c *options) error {
		c.responsePrefix = responsePrefix
		return nil
	}
}

func WithJSONSchema(jsonSchema *string) Option {
	return func(c *options) error {
		c.jsonSchema = jsonSchema
		return nil
	}
}

func WithJSONMode(jsonMode bool) Option {
	return func(c *options) error {
		c.jsonMode = jsonMode
		return nil
	}
}

func WithEscapedSpecialTokens(specialTokens []string) Option {
	return func(c *options) error {
		re := regexp.MustCompile(`([<>|])`)
		for _, token := range specialTokens {
			replacement := re.ReplaceAllString(token, `\$1`)
			c.escapedSpecialTokens[token] = replacement
		}
		return nil
	}
}

func WithAdditionalTemplateFields(fields map[string]any) Option {
	return func(c *options) error {
		c.additionalTemplateFields = fields
		return nil
	}
}
