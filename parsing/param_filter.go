package parsing

import (
	"encoding/json"
	"strings"
	"unicode"
)

type ParamState int

const (
	Beginning ParamState = iota
	ComplexType
	BasicType
	End
)

/*
This is the code for handling a JSON parameter value
The parameter value can be of different types

	a string, an object, an array - below this is called a complex type
	a boolean, null or a number - below this is called a basic type

We are expecting for the end of the param value with , or }
Then we decide the next mode either another parameter name or the end of the tool params
*/
func (f *filter) HandleParamValue(str string) ([]FilterOutput, int) {
	if str == "" {
		return nil, 0
	}
	switch f.actionMetaData.curParamState {
	case Beginning:
		return f.HandleParamValueBeginning(str)
	case ComplexType:
		return f.HandleParamValueComplexType(str)
	case BasicType:
		return f.HandleParamValueBasicType(str)
	case End:
		return f.HandleParamValueEndType(str)
	}
	return nil, 0
}

// We decide the type of a param (basic or complex bases on the first character of the value)
func (f *filter) HandleParamValueBeginning(str string) ([]FilterOutput, int) {
	trim := strings.TrimLeftFunc(str, unicode.IsSpace)
	switch {
	case trim == "":
		return nil, 0
	case trim[0] == '"' || trim[0] == '{' || trim[0] == '[':
		f.actionMetaData.curParamState = ComplexType
		return f.HandleParamValue(str)
	case trim[0] == '}' || trim[0] == ',':
		f.actionMetaData.curParamState = End
		return f.HandleParamValue(str)
	default:
		f.actionMetaData.curParamState = BasicType
		return f.HandleParamValue(str)
	}
}

// With a basic type we just look for the ending and send everything in between
func (f *filter) HandleParamValueBasicType(str string) ([]FilterOutput, int) {
	idx, _ := findPartial(str, []string{"}", ","})
	if idx == -1 {
		return f.sendParamValueChunk(str)
	}
	out, _ := f.sendParamValueChunk(str[:idx])
	f.actionMetaData.curParamState = End
	o, r := f.HandleParamValue(str[idx:])
	return append(out, o...), r + len(str[:idx])
}

// In complex we build up a buffer - as soon as it is valid JSON we go to end
// Otherwise we just send it as a string and add to buffer
func (f *filter) HandleParamValueComplexType(str string) ([]FilterOutput, int) {
	idx := findValidJSONValue(f.actionMetaData.paramValueBuffer, str)
	if idx == -1 {
		// If we don't find valid json then return the whole string and wait for more
		out, rem := f.sendParamValueChunk(str)
		f.actionMetaData.paramValueBuffer += str
		return out, rem
	}

	// We have a valid JSON value
	f.actionMetaData.paramValueBuffer = ""
	f.actionMetaData.curParamState = End
	out, _ := f.sendParamValueChunk(str[:idx])
	o, r := f.HandleParamValue(str[idx:])
	return append(out, o...), r + len(str[:idx])
}

// We are at the end of the param value - we decide the next mode
func (f *filter) HandleParamValueEndType(str string) ([]FilterOutput, int) {
	trim := strings.TrimLeftFunc(str, unicode.IsSpace)
	if trim == "" {
		return nil, 0
	}
	// Send anything before the end char
	idx := strings.Index(str, string(trim[0]))
	trimSend := strings.TrimRightFunc(str[:idx], unicode.IsSpace)
	out, rem := f.sendParamValueChunk(trimSend)

	// Reset all the metadata
	f.actionMetaData.trimLeft = true
	f.actionMetaData.paramValueBuffer = ""
	f.actionMetaData.curParamState = Beginning
	f.actionMetaData.curParamName = ""

	if trim[0] == '}' {
		// End of the all the parameters - end of the tool
		f.actionMetaData.mode = ToolEnd
		f.actionMetaData.curToolIndex++
	} else {
		// End of the parameter - next is the parameter name
		f.actionMetaData.mode = ParamValueEnd
	}
	o, r := f.ParseActions(str[rem+1:])
	return append(out, o...), r + len(str[:rem+1]) // +1 for the , or }
}

// find the index of the first valid json prefix
func findValidJSONValue(buffer string, str string) int {
	wholeStr := buffer
	for i, c := range str {
		wholeStr += string(c)
		if json.Valid([]byte(wholeStr)) {
			return i + 1
		}
	}
	return -1
}
