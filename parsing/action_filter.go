package parsing

import (
	"regexp"
	"strings"
	"unicode"
)

type actionMode struct{ e uint }

var (
	notStarted    = actionMode{0}
	toolCallID    = actionMode{1}
	toolCallIDEnd = actionMode{2}
	toolName      = actionMode{3}
	toolNameEnd   = actionMode{4}
	paramName     = actionMode{5}
	paramValue    = actionMode{6}
	toolEnd       = actionMode{7}
	paramNameEnd  = actionMode{8}
	paramValueEnd = actionMode{9}
	rawParam      = actionMode{10}
)

type filterAction struct {
	mode             actionMode
	curToolCallIndex int
	trimLeft         bool

	// Parameter metadata
	curParamName     string
	curParamState    paramState
	paramValueBuffer string
}

var (
	toolCallIDRegex = regexp.MustCompile(`"tool_call_id":\s*"`)
	toolNameRegex   = regexp.MustCompile(`"tool_name":\s*"`)
	paramRegex      = regexp.MustCompile(`"parameters":\s*{\s*"`)
	rawParamRegex   = regexp.MustCompile(`"parameters":\s*`)
	paramNameRegex  = regexp.MustCompile(`\s*:\s*`)

	llamaToolNameRegex = regexp.MustCompile(`"name":\s*"`)
)

func (f *filter) ParseActions(str string) ([]FilterOutput, int) {
	// If the last character is \ then we are waiting for more - incase of an escaped character
	if str == "" || str[len(str)-1] == '\\' {
		return nil, 0
	}
	switch f.actionMetaData.mode {
	case notStarted, toolEnd:
		return f.HandleBeforeTool(str, f.hasToolCallID)
	case toolCallID:
		return f.HandleInToolCallID(str)
	case toolCallIDEnd:
		return f.HandleToolCallIDEnd(str)
	case toolName:
		return f.HandleInToolName(str)
	case toolNameEnd:
		return f.HandleToolNameEnd(str)
	case rawParam:
		return f.HandleRawParam(str)
	case paramName:
		return f.HandleParamName(str)
	case paramNameEnd:
		return f.HandleEndOfParamName(str)
	case paramValue:
		return f.HandleParamValue(str)
	case paramValueEnd:
		return f.HandleParamValueEnd(str)
	}
	return nil, 0
}

// We are waiting for "tool_name": " (or tool_call_id) the regex allows all the whitespace in between
func (f *filter) HandleBeforeTool(str string, checkCallID bool) ([]FilterOutput, int) {
	var indices []int
	var mode actionMode

	switch {
	case f.llamaToolParsing:
		indices = llamaToolNameRegex.FindStringIndex(str)
		mode = toolName
	case checkCallID:
		indices = toolCallIDRegex.FindStringIndex(str)
		mode = toolCallID
	default:
		indices = toolNameRegex.FindStringIndex(str)
		mode = toolName
	}
	if indices == nil {
		return nil, 0
	}
	f.actionMetaData.mode = mode
	f.actionMetaData.trimLeft = true // Start with trimming left
	out, rem := f.ParseActions(str[indices[1]:])
	return out, rem + len(str[:indices[1]])
}

// We are expecting for the end of the tool call id with "
func (f *filter) HandleInToolCallID(str string) ([]FilterOutput, int) {
	idx := findNonEscapedChar(str, '"')
	if idx == -1 {
		// Wait till end of quotes
		return nil, 0
	}
	out := f.sendToolCallIDChunk(str[:idx])
	f.actionMetaData.mode = toolCallIDEnd
	o, r := f.ParseActions(str[idx:]) // one for the quote
	return append(out, o...), r + len(str[:idx]) + 1
}

func (f *filter) HandleToolCallIDEnd(str string) ([]FilterOutput, int) {
	// Looking for the name next
	return f.HandleBeforeTool(str, false)
}

// We are expecting for the end of the tool name with "
func (f *filter) HandleInToolName(str string) ([]FilterOutput, int) {
	idx := findNonEscapedChar(str, '"')
	if idx == -1 {
		// Wait till end of tool name
		return nil, 0
	}
	out := f.sendToolNameChunk(str[:idx])
	f.actionMetaData.mode = toolNameEnd
	o, r := f.ParseActions(str[idx:])
	return append(out, o...), r + len(str[:idx]) + 1 // one for the quote
}

// We are waiting for "parameters": { " the regex allows all the whitespace in between
func (f *filter) HandleToolNameEnd(str string) ([]FilterOutput, int) {
	indices := paramRegex.FindStringIndex(str)
	if indices == nil {
		idx := strings.Index(str, `}`) // no parameters (directly answer for example)
		if idx == -1 {
			return nil, 0
		}
		f.actionMetaData.mode = toolEnd
		f.actionMetaData.curToolCallIndex++
		f.actionMetaData.curParamName = ""
		out, rem := f.ParseActions(str[idx:])
		return out, rem + len(str[:idx])
	}
	if f.streamProcessedParams {
		f.actionMetaData.mode = paramName
		out, rem := f.ParseActions(str[indices[1]:])
		return out, rem + len(str[:indices[1]])
	}
	f.actionMetaData.mode = rawParam
	// Without {
	indices = rawParamRegex.FindStringIndex(str)
	out, rem := f.ParseActions(str[indices[1]:])
	return out, rem + len(str[:indices[1]])
}

// When handling the parameters raw we just send the whole string
// Waiting for a valid JSON parameter to know we are at the end of the param
func (f *filter) HandleRawParam(str string) ([]FilterOutput, int) {
	idx := findValidJSONValue(f.actionMetaData.paramValueBuffer, str)
	if idx == -1 {
		// If we don't find valid json then return the whole string and wait for more
		out := f.sendRawParamChunkWithoutIndentation(str)
		f.actionMetaData.paramValueBuffer += str
		return out, len(str)
	}

	// We have a valid JSON value
	out := f.sendRawParamChunkWithoutIndentation(str[:idx])
	f.actionMetaData.paramValueBuffer = ""
	f.actionMetaData.curToolCallIndex++
	f.actionMetaData.mode = toolEnd
	o, r := f.ParseActions(str[idx:])
	return append(out, o...), r + len(str[:idx])
}

// NumSpaceToRemovePerLine is the number of spaces to remove from each line when streaming raw parameters.
// (excluding the first line which is just "{\n"). This is exactly two levels of indentation.
const NumSpaceToRemovePerLine = 8

func (f *filter) sendRawParamChunkWithoutIndentation(str string) []FilterOutput {
	trimmedStrBldr := strings.Builder{}
	for _, c := range str {
		switch {
		case c == '\n':
			f.rawParamIndentLengthRemoved = 0
			f.sawNonWhitespaceInCurrentline = false
		case unicode.IsSpace(c):
			if f.rawParamIndentLengthRemoved < NumSpaceToRemovePerLine && !f.sawNonWhitespaceInCurrentline {
				f.rawParamIndentLengthRemoved++
				continue
			}
		default:
			f.sawNonWhitespaceInCurrentline = true
		}
		trimmedStrBldr.WriteRune(c)
	}
	return f.sendRawParamChunk(trimmedStrBldr.String())
}

// We are expecting for the end of the param name with "
func (f *filter) HandleParamName(str string) ([]FilterOutput, int) {
	idx := findNonEscapedChar(str, '"')
	if idx == -1 {
		// Wait till end of parameter name
		return nil, 0
	}
	out := f.sendParamNameChunk(str[:idx])
	f.actionMetaData.mode = paramNameEnd
	o, r := f.ParseActions(str[idx:])
	return append(out, o...), r + len(str[:idx]) + 1 // one for the quote
}

// We are expecting : for the start of the value the regex allows for whitespace in between
func (f *filter) HandleEndOfParamName(str string) ([]FilterOutput, int) {
	indices := paramNameRegex.FindStringIndex(str)
	if indices == nil {
		return nil, 0
	}
	f.actionMetaData.mode = paramValue
	out, rem := f.ParseActions(str[indices[1]:])
	return out, rem + len(str[:indices[1]])
}

// We are expecting for the start of the param name with " this is only when we have a next param
func (f *filter) HandleParamValueEnd(str string) ([]FilterOutput, int) {
	idx := strings.Index(str, `"`)
	if idx == -1 {
		return nil, 0
	}
	f.actionMetaData.mode = paramName
	out, rem := f.ParseActions(str[idx+1:])
	return out, rem + len(str[:idx]) + 1
}

func (f *filter) sendToolCallIDChunk(str string) []FilterOutput {
	if str == "" || !f.streamToolActions {
		return nil
	}
	return []FilterOutput{{
		ToolCalls: &FilterToolCallDelta{
			Index: f.actionMetaData.curToolCallIndex,
			ID:    str,
		},
	}}
}

func (f *filter) sendToolNameChunk(str string) []FilterOutput {
	if str == "" || !f.streamToolActions {
		return nil
	}
	return []FilterOutput{{
		ToolCalls: &FilterToolCallDelta{
			Index: f.actionMetaData.curToolCallIndex,
			Name:  str,
		},
	}}
}

func (f *filter) sendParamNameChunk(str string) []FilterOutput {
	if str == "" || !f.streamToolActions {
		return nil
	}
	f.actionMetaData.curParamName = str
	return []FilterOutput{{
		ToolCalls: &FilterToolCallDelta{
			Index: f.actionMetaData.curToolCallIndex,
			ParamDelta: &FilterToolParameter{
				Name: str,
			},
		},
	}}
}

func (f *filter) sendRawParamChunk(str string) []FilterOutput {
	if str == "" || !f.streamToolActions {
		return nil
	}
	return []FilterOutput{{
		ToolCalls: &FilterToolCallDelta{
			Index:         f.actionMetaData.curToolCallIndex,
			RawParamDelta: str,
		},
	}}
}

func (f *filter) sendParamValueChunk(str string) ([]FilterOutput, int) {
	// Just for parameter value we need to trim the right whitespace as the value doesn't have a defined end like "
	trimmedStr := strings.TrimRightFunc(str, unicode.IsSpace)
	// Trim left just at the start
	if f.actionMetaData.trimLeft {
		trimmedStr = strings.TrimLeftFunc(trimmedStr, unicode.IsSpace)
	}
	if trimmedStr == "" || !f.streamToolActions {
		return nil, 0
	}
	// We have seen a value so stop trimming left
	f.actionMetaData.trimLeft = false
	return []FilterOutput{{
		ToolCalls: &FilterToolCallDelta{
			Index: f.actionMetaData.curToolCallIndex,
			ParamDelta: &FilterToolParameter{
				Name:       f.actionMetaData.curParamName,
				ValueDelta: trimmedStr,
			},
		},
	}}, len(str)
}

func findNonEscapedChar(str string, char byte) int {
	// there needs to be an even number of \ before the character
	// if there is an odd number then the character is escaped
	for i := 0; i < len(str); i++ {
		if str[i] == char {
			// check if the character is escaped
			escaped := false
			for j := i - 1; j >= 0; j-- {
				if str[j] != '\\' {
					break
				}
				escaped = !escaped
			}
			if !escaped {
				return i
			}
		}
	}
	return -1
}
