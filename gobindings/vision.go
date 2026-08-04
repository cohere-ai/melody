package gobindings

import (
	"errors"
	"unsafe"
)

// #include <stdlib.h>
// #include "melody.h"
import "C"

// VisionGeneration is the unary parse result for parse-/vision-model generations that
// interleave markdown with [visual_element] blocks.
type VisionGeneration struct {
	Segments []VisionSegment
}

// VisionSegmentKind identifies a segment as text or a structured element.
type VisionSegmentKind int

const (
	VisionSegmentKindText VisionSegmentKind = iota
	VisionSegmentKindElement
)

// VisionSegment is either prose or a structured vision element.
type VisionSegment struct {
	Kind    VisionSegmentKind
	Text    string
	Element *VisionElement
}

// VisionElement is the structured content of a [visual_element] block.
type VisionElement struct {
	Type        string
	BBox        *VisionBBox
	Description *string
	Title       *string
	HTML        *string
	Extra       map[string]string
}

// VisionBBox is a pixel bounding box from a bbox: field.
type VisionBBox struct {
	TopLeftX     int
	TopLeftY     int
	BottomRightX int
	BottomRightY int
}

// ParseVisionGeneration parses a complete parse-model generation (unary, not streaming).
func ParseVisionGeneration(text string) (*VisionGeneration, error) {
	cText := C.CString(text)
	defer C.free(unsafe.Pointer(cText))

	res := C.melody_parse_vision_generation(cText)
	if res == nil {
		return nil, errors.New("melody_parse_vision_generation returned null result struct")
	}
	defer C.melody_vision_generation_free(res)

	if res.error != nil {
		return nil, errors.New(C.GoString(res.error))
	}
	if res.result == nil {
		return nil, errors.New("melody_parse_vision_generation returned neither result nor error")
	}

	return convertCVisionGeneration(res.result), nil
}

func convertCVisionGeneration(cGen *C.CVisionGeneration) *VisionGeneration {
	out := &VisionGeneration{}
	if cGen.segments == nil || cGen.segments_len == 0 {
		return out
	}
	cSegs := unsafe.Slice(cGen.segments, int(cGen.segments_len))
	out.Segments = make([]VisionSegment, len(cSegs))
	for i, cs := range cSegs {
		out.Segments[i] = convertCVisionSegment(cs)
	}
	return out
}

func convertCVisionSegment(cs C.CVisionSegment) VisionSegment {
	switch cs.kind {
	case C.CVisionSegmentKind_Text:
		return VisionSegment{
			Kind: VisionSegmentKindText,
			Text: C.GoString(cs.text),
		}
	case C.CVisionSegmentKind_Element:
		seg := VisionSegment{Kind: VisionSegmentKindElement}
		if cs.element != nil {
			seg.Element = convertCVisionElement(cs.element)
		}
		return seg
	default:
		return VisionSegment{Kind: VisionSegmentKindText, Text: C.GoString(cs.text)}
	}
}

func convertCVisionElement(ce *C.CVisionElement) *VisionElement {
	el := &VisionElement{
		Type: C.GoString(ce.element_type),
	}
	if ce.bbox != nil {
		el.BBox = &VisionBBox{
			TopLeftX:     int(ce.bbox.top_left_x),
			TopLeftY:     int(ce.bbox.top_left_y),
			BottomRightX: int(ce.bbox.bottom_right_x),
			BottomRightY: int(ce.bbox.bottom_right_y),
		}
	}
	if ce.description != nil {
		s := C.GoString(ce.description)
		el.Description = &s
	}
	if ce.title != nil {
		s := C.GoString(ce.title)
		el.Title = &s
	}
	if ce.html != nil {
		s := C.GoString(ce.html)
		el.HTML = &s
	}
	if ce.extra != nil && ce.extra_len > 0 {
		fields := unsafe.Slice(ce.extra, int(ce.extra_len))
		el.Extra = make(map[string]string, len(fields))
		for _, f := range fields {
			el.Extra[C.GoString(f.key)] = C.GoString(f.value)
		}
	}
	return el
}
