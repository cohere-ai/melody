package gobindings

import (
	"encoding/json"
	"errors"
	"unsafe"
)

// #include <stdlib.h>
// #include "melody.h"
import "C"

// VisionGeneration is the unary parse result for parse-/vision-model generations that
// interleave markdown with [visual_element] blocks.
type VisionGeneration struct {
	Segments []VisionSegment `json:"segments"`
}

// VisionSegment is either prose or a structured vision element.
// Kind is "text" or "element".
type VisionSegment struct {
	Kind    string         `json:"kind"`
	Text    string         `json:"text,omitempty"`
	Element *VisionElement `json:"element,omitempty"`
}

// VisionElement is the structured content of a [visual_element] block.
type VisionElement struct {
	Type        string            `json:"type"`
	BBox        *VisionBBox       `json:"bbox,omitempty"`
	Description *string           `json:"description,omitempty"`
	Title       *string           `json:"title,omitempty"`
	HTML        *string           `json:"html,omitempty"`
	Extra       map[string]string `json:"extra,omitempty"`
}

// VisionBBox is a pixel bounding box from a bbox: field.
type VisionBBox struct {
	TopLeftX     int `json:"top_left_x"`
	TopLeftY     int `json:"top_left_y"`
	BottomRightX int `json:"bottom_right_x"`
	BottomRightY int `json:"bottom_right_y"`
}

// ParseVisionGeneration parses a complete parse-model generation (unary, not streaming).
func ParseVisionGeneration(text string) (*VisionGeneration, error) {
	cText := C.CString(text)
	defer C.free(unsafe.Pointer(cText))

	res := C.melody_parse_vision_generation(cText)
	if res == nil {
		return nil, errors.New("melody_parse_vision_generation returned null result struct")
	}
	defer C.melody_render_result_free(res)

	if res.error != nil {
		return nil, errors.New(C.GoString(res.error))
	}
	if res.result == nil {
		return nil, errors.New("melody_parse_vision_generation returned neither result nor error")
	}

	var parsed VisionGeneration
	if err := json.Unmarshal([]byte(C.GoString(res.result)), &parsed); err != nil {
		return nil, err
	}
	return &parsed, nil
}
