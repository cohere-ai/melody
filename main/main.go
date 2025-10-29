package main

import (
	"C"
	"sync"
	"unsafe"

	"github.com/google/uuid"
	"gitlab.com/pygolo/py"

	"github.com/cohere-ai/melody"
	"github.com/cohere-ai/melody/parsing"
)

var registry sync.Map

type PythonFilter struct {
	ID string
}

func NewFilter(o parsing.Options) PythonFilter {
	id := uuid.New().String()
	parsing.HandleMultiHopCmd3()(&o)
	parsing.StreamToolActions()(&o)
	registry.Store(id, parsing.NewFromOptions(nil, nil, o))
	return PythonFilter{ID: id}
}

func CloseFilter(p PythonFilter) []parsing.FilterOutput {
	fAny, ok := registry.Load(p.ID)
	if !ok {
		return nil
	}
	f, ok := fAny.(parsing.Filter)
	if !ok {
		return nil
	}
	registry.Delete(p.ID)
	return f.FlushPartials()
}

func WriteDecoded(p PythonFilter, decodedToken string) []parsing.FilterOutput {
	fAny, ok := registry.Load(p.ID)
	if !ok {
		return nil
	}
	f, ok := fAny.(parsing.Filter)
	if !ok {
		return nil
	}
	return f.WriteDecoded(decodedToken, parsing.TokenIDsWithLogProb{})
}

func myext(Py py.Py, m py.Object) error {
	// Doc string
	if err := Py.Module_SetDocString(m, "Melody provides templating and parsing for Cohere models."); err != nil {
		return err
	}
	// Register funcs
	if err := Py.Object_SetAttr(m, "NewFilter", NewFilter); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "CloseFilter", CloseFilter); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "WriteDecoded", WriteDecoded); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "NewOptions", parsing.NewOptions); err != nil {
		return err
	}

	// Register Structs
	{
		if err := Py.GoRegisterStruct(melody.Message{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(melody.Message{})
		if err := Py.Object_SetAttr(m, "Message", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(melody.Tool{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(melody.Tool{})
		if err := Py.Object_SetAttr(m, "Tool", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(melody.Content{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(melody.Content{})
		if err := Py.Object_SetAttr(m, "Content", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(melody.Image{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(melody.Image{})
		if err := Py.Object_SetAttr(m, "Image", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(melody.ToolCall{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(melody.ToolCall{})
		if err := Py.Object_SetAttr(m, "ToolCall", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(parsing.TokenIDsWithLogProb{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(parsing.TokenIDsWithLogProb{})
		if err := Py.Object_SetAttr(m, "TokenIDsWithLogProb", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(parsing.FilterOutput{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(parsing.FilterOutput{})
		if err := Py.Object_SetAttr(m, "FilterOutput", t); err != nil {
			return err
		}
	}
	{ // deprecated: remove pls
		if err := Py.GoRegisterStruct(parsing.FilterSearchQueryDelta{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(parsing.FilterSearchQueryDelta{})
		if err := Py.Object_SetAttr(m, "FilterSearchQueryDelta", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(parsing.FilterToolCallDelta{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(parsing.FilterToolCallDelta{})
		if err := Py.Object_SetAttr(m, "FilterToolCallDelta", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(parsing.FilterToolParameter{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(parsing.FilterToolParameter{})
		if err := Py.Object_SetAttr(m, "FilterToolParameter", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(parsing.FilterCitation{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(parsing.FilterCitation{})
		if err := Py.Object_SetAttr(m, "FilterCitation", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(parsing.Source{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(parsing.Source{})
		if err := Py.Object_SetAttr(m, "Source", t); err != nil {
			return err
		}
	}
	{
		if err := Py.GoRegisterStruct(parsing.Options{}); err != nil {
			return err
		}
		t, _ := Py.GoGetStructType(parsing.Options{})
		if err := Py.Object_SetAttr(m, "Options", t); err != nil {
			return err
		}
	}

	// Conversion for ordered json
	if err := py.GoRegisterConversions(orderedJSONObjectConversion); err != nil {
		return err
	}

	// Register conversions
	if err := py.GoRegisterConversions(pythonFilterConversion); err != nil {
		return err
	}
	if err := py.GoRegisterConversions(roleConversion); err != nil {
		return err
	}
	if err := py.GoRegisterConversions(contentTypeConversion); err != nil {
		return err
	}
	if err := py.GoRegisterConversions(citationQualityConversion); err != nil {
		return err
	}
	if err := py.GoRegisterConversions(groundingConversion); err != nil {
		return err
	}
	if err := py.GoRegisterConversions(safetyModeConversion); err != nil {
		return err
	}
	if err := py.GoRegisterConversions(reasoningTypeConversion); err != nil {
		return err
	}
	if err := py.GoRegisterConversions(filterModeConversion); err != nil {
		return err
	}
	if err := py.GoRegisterConversions(filterToolCallDeltaPointerConversion); err != nil {
		return err
	}
	if err := py.GoRegisterConversions(filterToolParameterPointerConversion); err != nil {
		return err
	}

	// Register enums
	if err := Py.Object_SetAttr(m, "UserRole", melody.UserRole); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "ChatbotRole", melody.ChatbotRole); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "ToolRole", melody.ToolRole); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "SystemRole", melody.SystemRole); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "TextContentType", melody.TextContentType); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "ThinkingContentType", melody.ThinkingContentType); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "ImageContentType", melody.ImageContentType); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "OnCitationQuality", melody.OnCitationQuality); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "OffCitationQuality", melody.OffCitationQuality); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "EnabledGrounding", melody.EnabledGrounding); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "DisabledGrounding", melody.DisabledGrounding); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "NoneSafetyMode", melody.NoneSafetyMode); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "StrictSafetyMode", melody.StrictSafetyMode); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "ContextualSafetyMode", melody.ContextualSafetyMode); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "EnabledReasoningType", melody.EnabledReasoningType); err != nil {
		return err
	}
	if err := Py.Object_SetAttr(m, "DisabledReasoningType", melody.DisabledReasoningType); err != nil {
		return err
	}
	return nil
}

//export PyInit_melody
func PyInit_melody() unsafe.Pointer {
	return py.GoExtend(myext)
}

// required by cgo but unused
func main() {
}
