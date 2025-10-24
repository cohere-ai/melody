package main

import (
	"C"
	"unsafe"

	"gitlab.com/pygolo/py"

	"github.com/cohere-ai/melody"
)

func myext(Py py.Py, m py.Object) error {
	// Doc string
	if err := Py.Module_SetDocString(m, "Melody provides templating and parsing for Cohere models."); err != nil {
		return err
	}
	// Register funcs
	if err := Py.Object_SetAttr(m, "test", Test); err != nil {
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

	// Conversion for ordered json
	if err := py.GoRegisterConversions(orderedJSONObjectConversion); err != nil {
		return err
	}

	// Register conversions
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
