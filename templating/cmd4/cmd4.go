// Package cmd4 provides a renderer for the Cmd4 model.
package cmd4

import (
	"maps"

	"github.com/osteele/liquid"

	"github.com/cohere-ai/melody"
	"github.com/cohere-ai/melody/templating"
)

var (
	templateEngine = liquid.NewEngine()
)

func Render(m []melody.Message, os ...Option) (string, error) {
	r := options{escapedSpecialTokens: make(map[string]string)}
	for _, o := range os {
		err := o(&r)
		if err != nil {
			return "", err
		}
	}

	templateTools, err := templating.ToolsToTemplate(r.availableTools)
	if err != nil {
		return "", err
	}

	var messages []map[string]any
	var docs []string
	messages, err = templating.MessagesToTemplate(m, len(r.documents) > 0, r.escapedSpecialTokens)
	if err != nil {
		return "", err
	}
	for _, d := range r.documents {
		docs = append(docs, templating.EscapeSpecialTokens(d, r.escapedSpecialTokens))
	}
	substitutions := map[string]any{}
	for k, v := range r.additionalTemplateFields {
		substitutions[k] = v
	}
	maps.Copy(substitutions, map[string]any{
		"developer_instruction":         r.devInstruction,
		"platform_instruction_override": r.platformInstruction,
		"messages":                      messages,
		"documents":                     docs,
		"available_tools":               templateTools,
		"grounding":                     r.grounding.String(),
		"response_prefix":               r.responsePrefix,
		"json_schema":                   r.jsonSchema,
		"json_mode":                     r.jsonMode,
	})

	return templateEngine.ParseAndRenderString(r.template, templating.SafeLiquidSubstitutions(substitutions))
}
func (r *options) ProcessToken(_ int64) error {
	return nil
}
