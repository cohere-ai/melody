// Package templating contains templating logic for Cohere models
package templating

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"maps"
	"reflect"
	"sort"
	"strconv"
	"strings"

	"github.com/cohere-ai/melody"
	"github.com/cohere-ai/melody/lib/orderedjson"
)

const DocumentIDKey = "id"
const ExcludesFieldKey = "_excludes"

// SafeLiquidSubstitutions converts any null pointer to a null interface.
// An interface = nil is not the same as an interface = variable_of_type_pointer_that_is_nil
// Liquid doesn't support nil values (pointers), but a nil interface is valid.
// TODO: Fix the underlying library to support nil pointers.
func SafeLiquidSubstitutions(sub map[string]any) map[string]any {
	for k, v := range sub {
		if v == nil {
			continue
		}
		if reflect.ValueOf(v).Kind() == reflect.Ptr {
			if reflect.ValueOf(v).IsNil() {
				sub[k] = nil
			}
		}
	}
	return sub
}

// AddSpacesToJSONEncoding adds spacing to a json string how command 3 models expect:
// Spaces after colons in maps, and after commas.
func AddSpacesToJSONEncoding(input string) string {
	var b strings.Builder
	b.Grow(len(input))
	inStringLiteral := false
	lastRuneIsBackslash := false
	for _, r := range input {
		b.WriteRune(r)
		if !inStringLiteral && (r == ',' || r == ':') {
			b.WriteRune(' ')
		}
		if r == '"' && !lastRuneIsBackslash {
			inStringLiteral = !inStringLiteral
		}
		lastRuneIsBackslash = r == '\\' && !lastRuneIsBackslash
	}
	return b.String()
}

// MarshalJSONFormatted will marshal the schema to json with the fields formatted for the prompt template with specific spacing
func MarshalJSONFormatted(v any) ([]byte, error) {
	schemaString, err := json.MarshalIndent(v, "", "")
	if err != nil {
		return nil, err
	}

	// prompt template expects to get key values seperated with a space
	// json.MarshalIndent will do this, but it will also add newlines, which we want to remove
	// this is a hacky way to remove the new lines while keeping the spaces
	// todo look into doing this in a custom marshal function
	schemaStringSpaced := strings.ReplaceAll(string(schemaString), "{\n", "{")
	schemaStringSpaced = strings.ReplaceAll(schemaStringSpaced, "\n}", "}")
	schemaStringSpaced = strings.ReplaceAll(schemaStringSpaced, "[\n", "[")
	schemaStringSpaced = strings.ReplaceAll(schemaStringSpaced, "\n]", "]")
	schemaStringSpaced = strings.ReplaceAll(schemaStringSpaced, "\n", " ")

	return []byte(schemaStringSpaced), nil
}

func JSONEscapeString(s string) string {
	b, err := json.Marshal(s)
	if err != nil {
		return ""
	}
	if len(b) < 2 { // There must be atleast an open an closing quote
		return ""
	}
	return string(b[1 : len(b)-1])
}

type templateContent struct {
	Type string
	Data string
}
type templateToolResult struct {
	ToolCallID int
	Documents  []string
}
type templateMessage struct {
	Role        string
	ToolCalls   []string
	Content     []templateContent
	ToolResults []templateToolResult

	AdditionalFields map[string]any
}

func messageToMap(ms []templateMessage) []map[string]any {
	mapped := make([]map[string]any, len(ms))
	for i, m := range ms {
		mapped[i] = make(map[string]any)
		maps.Copy(mapped[i], SafeLiquidSubstitutions(m.AdditionalFields))
		maps.Copy(mapped[i], map[string]any{
			"role":         m.Role,
			"tool_calls":   m.ToolCalls,
			"content":      contentToMap(m.Content),
			"tool_results": toolResultToMap(m.ToolResults),
		})
	}
	return mapped
}
func contentToMap(cs []templateContent) []map[string]any {
	mapped := make([]map[string]any, len(cs))
	for i, c := range cs {
		mapped[i] = map[string]any{
			"type": c.Type,
			"data": c.Data,
		}
	}
	return mapped
}
func toolResultToMap(trs []templateToolResult) []map[string]any {
	mapped := make([]map[string]any, len(trs))
	for i, tr := range trs {
		mapped[i] = map[string]any{
			"tool_call_id": tr.ToolCallID,
			"documents":    tr.Documents,
		}
	}
	return mapped
}

func EscapeSpecialTokens(text string, specialTokenMap map[string]string) string {
	for specialToken, replacement := range specialTokenMap {
		text = strings.ReplaceAll(text, specialToken, replacement)
	}
	return text
}

// MessagesToTemplate turns a message into a map that can be used with Command 3 templates. The map needs to be backwards compatible with all the templates that are defined (i.e. don't remove keys).
func MessagesToTemplate(messages []melody.Message, docsPresent bool, specialTokenMap map[string]string) ([]map[string]any, error) {
	templateMessages := []templateMessage{}
	runningToolCallIdx := 0
	if docsPresent {
		runningToolCallIdx = 1
	}
	var err error
	var toolCallIDToToolResultIdx = make(map[string]int) // tracks which template "tool result" contains the documents for a tool call id
	var toolCallIDToPromptID = make(map[string]int)      // tracks the assignment of auto-incrementing ids -> tool calls
	for i, msg := range messages {
		if msg.Role == melody.ToolRole {
			// Template expects all tool messages to be aggregated. The template expects each tool message
			// to contain an array of tool results which maps 1 to 1 with tool calls. Each tool result
			// contains a tool call id and a list of documents (which are individually citable).

			// Melody accepts multiple tool messages - we concatenate all of a tool message's content
			// into a single document and add it to the relevant tool result.
			toolCallID := msg.ToolCallID
			if toolCallID == "" {
				return nil, fmt.Errorf("tool message[%d] missing tool_call_id", i)
			}

			toolCallTemplateID, ok := toolCallIDToPromptID[toolCallID]
			if !ok {
				// Really this should be an error but v1 tool use didn't enforce this so we have to create a prompt id
				// return nil, fmt.Errorf("tool message[%d] has unknown tool_call_id: %s", i, toolCallID)
				toolCallID = strconv.Itoa(runningToolCallIdx)
				runningToolCallIdx++
			}

			// Aggregate text content into single string
			toolDocument := strings.Builder{}
			for j, contentItem := range msg.Content {
				if contentItem.Type != melody.TextContentType {
					return nil, fmt.Errorf("tool message[%d].content[%d] invalid content type: %s", i, j, contentItem.Type)
				}
				_, err = toolDocument.WriteString(contentItem.Text)
				if err != nil {
					return nil, fmt.Errorf("failed to write tool message[%d].content[%d] text: %w", i, j, err)
				}
			}

			// Insert a tool message to aggregate into (if its the first message, or we haven't inserted a tool message yet for this hop).
			if len(templateMessages) == 0 || templateMessages[len(templateMessages)-1].Role != melody.ToolRole.String() {
				templateMessages = append(templateMessages, templateMessage{
					Role:        msg.Role.String(),
					ToolResults: []templateToolResult{},
				})
			}

			m := &templateMessages[len(templateMessages)-1]

			// Insert a tool result if one doesn't exist
			toolResultIdx, ok := toolCallIDToToolResultIdx[toolCallID]
			if !ok {
				m.ToolResults = append(m.ToolResults, templateToolResult{ToolCallID: toolCallTemplateID})
				// Update toolCallIDToToolResultIdx
				toolResultIdx = len(m.ToolResults) - 1
				toolCallIDToToolResultIdx[toolCallID] = toolResultIdx
			}

			// Append the document to the tool result
			m.ToolResults[toolResultIdx].Documents = append(m.ToolResults[toolResultIdx].Documents, EscapeSpecialTokens(toolDocument.String(), specialTokenMap))

			continue // non-tool messages are a special case other Roles handled generally below
		}

		var templateMsgContent []templateContent
		for _, contentItem := range msg.Content {
			if contentItem.Type == melody.TextContentType {
				if msg.Role == melody.SystemRole {
					// dont escape special tokens for system messages
					templateMsgContent = append(templateMsgContent, templateContent{
						Type: "text",
						Data: contentItem.Text,
					})
				} else {
					templateMsgContent = append(templateMsgContent, templateContent{
						Type: "text",
						Data: EscapeSpecialTokens(contentItem.Text, specialTokenMap),
					})
				}
			}
			if contentItem.Type == melody.ThinkingContentType {
				if msg.Role == melody.ToolRole {
					return nil, errors.New("content type thinking is not supported for tool messages")
				}
				templateMsgContent = append(templateMsgContent, templateContent{
					Type: "thinking",
					Data: EscapeSpecialTokens(contentItem.Thinking, specialTokenMap),
				})
			}
			if contentItem.Type == melody.ImageContentType && contentItem.Image != nil {
				if msg.Role == melody.ToolRole {
					return nil, errors.New("content type image is not supported for tool messages")
				}
				templateMsgContent = append(templateMsgContent, templateContent{
					Type: "image",
					Data: contentItem.Image.TemplatePlaceholder,
				})
			}
		}

		var renderedToolCalls []string
		for _, tc := range msg.ToolCalls {
			if msg.Role != melody.ChatbotRole {
				return nil, errors.New("tool calls are only supported for chatbot/assistant messages")
			}
			if tc.ID == "" {
				return nil, fmt.Errorf("message[%d] has tool call with empty id", i)
			}
			_, ok := toolCallIDToPromptID[tc.ID]
			if ok {
				return nil, fmt.Errorf("message[%d] has duplicate tool call id: %s", i, tc.ID)
			}
			toolCallIDToPromptID[tc.ID] = runningToolCallIdx
			renderedToolCall, err := toolCallToTemplate(tc, runningToolCallIdx)
			if err != nil {
				return nil, fmt.Errorf("invalid message[%d] tool call: %w", i, err)
			}
			runningToolCallIdx++
			renderedToolCalls = append(renderedToolCalls, renderedToolCall)
		}

		templateMessages = append(templateMessages, templateMessage{
			Content:          templateMsgContent,
			Role:             msg.Role.String(),
			ToolCalls:        renderedToolCalls,
			AdditionalFields: msg.AdditionalFields,
		})
	}
	return messageToMap(templateMessages), nil
}

func toolCallToTemplate(tc melody.ToolCall, tcIndex int) (string, error) {
	// The tool call is represented in a specific way inside the prompt that elsewhere in the code / API (e.g. name vs tool_name).
	type toolCallTemplate struct {
		ToolCallID string             `json:"tool_call_id"`
		ToolName   string             `json:"tool_name"`
		Parameters orderedjson.Object `json:"parameters"`
	}

	toolCallRendered, err := json.Marshal(toolCallTemplate{
		ToolCallID: strconv.Itoa(tcIndex),
		ToolName:   tc.Name,
		Parameters: tc.Parameters,
	})

	if err != nil {
		return "", err
	}

	return AddSpacesToJSONEncoding(string(toolCallRendered)), nil
}

func DocumentMapToString(d map[string]any) (string, error) {
	// Build exclude set of document
	excludeSet := map[string]struct{}{DocumentIDKey: {}}
	excludesField, ok := d[ExcludesFieldKey]
	if ok {
		excludes, ok := excludesField.([]string)
		if ok {
			for _, e := range excludes {
				excludeSet[e] = struct{}{}
			}
		}
	}
	// sort keys
	var keys []string
	for k := range d {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	// Exclude fields
	fieldsToRender := map[string]any{}
	for _, k := range keys {
		if _, exclude := excludeSet[k]; !exclude {
			fieldsToRender[k] = d[k]
		}
	}
	// Render
	buf := new(bytes.Buffer)
	enc := json.NewEncoder(buf)
	enc.SetEscapeHTML(false)
	err := enc.Encode(&fieldsToRender)
	if err != nil {
		return "", err
	}
	docJSON := buf.String()
	// Encode adds newline by default, so must be removed before template rendering
	docJSON = strings.TrimRight(docJSON, "\n")
	return AddSpacesToJSONEncoding(docJSON), nil
}

func ToolsToTemplate(tools []melody.Tool) ([]map[string]any, error) {
	templateTools := make([]map[string]any, len(tools))
	for i, tool := range tools {
		formatedJSONSchema, err := MarshalJSONFormatted(tool.Parameters)
		if err != nil {
			return nil, err
		}
		templateTools[i] = map[string]any{
			"name": JSONEscapeString(tool.Name),
			"definition": map[string]any{
				"description": JSONEscapeString(tool.Description),
				"json_schema": string(formatedJSONSchema),
			},
		}
	}
	return templateTools, nil
}
