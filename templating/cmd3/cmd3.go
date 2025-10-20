// Package cmd3 provides a renderer for the cmd3 prompt template.
package cmd3

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
		"preamble":          r.devInstruction,
		"messages":          messages,
		"documents":         docs,
		"available_tools":   templateTools,
		"citation_mode":     r.citationQuality.String(),
		"safety_mode":       r.safetyMode.String(),
		"reasoning_options": map[string]bool{"enabled": r.reasoningType == melody.EnabledReasoningType},
		"skip_preamble":     r.skipPreamble,
		"skip_thinking":     r.reasoningType == melody.DisabledReasoningType,
		"response_prefix":   r.responsePrefix,
		"json_schema":       r.jsonSchema,
		"json_mode":         r.jsonMode,
	})

	return templateEngine.ParseAndRenderString(r.template, templating.SafeLiquidSubstitutions(substitutions))
}
func (r *options) ProcessToken(_ int64) error {
	return nil
}
