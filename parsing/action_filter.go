package parsing

import (
	"regexp"
	"strings"
	"unicode"
)

type ActionMode int

const (
	NotStarted ActionMode = iota
	ToolCallID
	ToolCallIDEnd
	ToolName
	ToolNameEnd
	ParamName
	ParamValue
	ToolEnd
	ParamNameEnd
	ParamValueEnd
	RawParam
)

type FilterAction struct {
	mode         ActionMode
	curToolIndex int
	trimLeft     bool

	// Parameter metadata
	curParamName     string
	curParamState    ParamState
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
	case NotStarted, ToolEnd:
		return f.HandleBeforeTool(str, f.hasToolCallID)
	case ToolCallID:
		return f.HandleInToolCallID(str)
	case ToolCallIDEnd:
		return f.HandleToolCallIDEnd(str)
	case ToolName:
		return f.HandleInToolName(str)
	case ToolNameEnd:
		return f.HandleToolNameEnd(str)
	case RawParam:
		return f.HandleRawParam(str)
	case ParamName:
		return f.HandleParamName(str)
	case ParamNameEnd:
		return f.HandleEndOfParamName(str)
	case ParamValue:
		return f.HandleParamValue(str)
	case ParamValueEnd:
		return f.HandleParamValueEnd(str)
	}
	return nil, 0
}

// We are waiting for "tool_name": " (or tool_call_id) the regex allows all the whitespace in between
func (f *filter) HandleBeforeTool(str string, checkCallID bool) ([]FilterOutput, int) {
	var indices []int
	var mode ActionMode

	switch {
	case f.llamaToolParsing:
		indices = llamaToolNameRegex.FindStringIndex(str)
		mode = ToolName
	case checkCallID:
		indices = toolCallIDRegex.FindStringIndex(str)
		mode = ToolCallID
	default:
		indices = toolNameRegex.FindStringIndex(str)
		mode = ToolName
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
	f.actionMetaData.mode = ToolCallIDEnd
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
	f.actionMetaData.mode = ToolNameEnd
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
		f.actionMetaData.mode = ToolEnd
		f.actionMetaData.curToolIndex++
		f.actionMetaData.curParamName = ""
		out, rem := f.ParseActions(str[idx:])
		return out, rem + len(str[:idx])
	}
	if f.streamProcessedParams {
		f.actionMetaData.mode = ParamName
		out, rem := f.ParseActions(str[indices[1]:])
		return out, rem + len(str[:indices[1]])
	}
	f.actionMetaData.mode = RawParam
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
	f.actionMetaData.curToolIndex++
	f.actionMetaData.mode = ToolEnd
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
	f.actionMetaData.mode = ParamNameEnd
	o, r := f.ParseActions(str[idx:])
	return append(out, o...), r + len(str[:idx]) + 1 // one for the quote
}

// We are expecting : for the start of the value the regex allows for whitespace in between
func (f *filter) HandleEndOfParamName(str string) ([]FilterOutput, int) {
	indices := paramNameRegex.FindStringIndex(str)
	if indices == nil {
		return nil, 0
	}
	f.actionMetaData.mode = ParamValue
	out, rem := f.ParseActions(str[indices[1]:])
	return out, rem + len(str[:indices[1]])
}

// We are expecting for the start of the param name with " this is only when we have a next param
func (f *filter) HandleParamValueEnd(str string) ([]FilterOutput, int) {
	idx := strings.Index(str, `"`)
	if idx == -1 {
		return nil, 0
	}
	f.actionMetaData.mode = ParamName
	out, rem := f.ParseActions(str[idx+1:])
	return out, rem + len(str[:idx]) + 1
}

func (f *filter) sendToolCallIDChunk(str string) []FilterOutput {
	if str == "" || !f.streamToolActions {
		return nil
	}
	return []FilterOutput{{
		ToolCalls: &FilterToolCallDelta{
			Index: f.actionMetaData.curToolIndex,
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
			Index: f.actionMetaData.curToolIndex,
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
			Index: f.actionMetaData.curToolIndex,
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
			Index:         f.actionMetaData.curToolIndex,
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
			Index: f.actionMetaData.curToolIndex,
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
