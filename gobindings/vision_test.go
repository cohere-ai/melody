package gobindings

import (
	"strings"
	"testing"
)

const pixtralSample = `[visual_element]

type: flowchart

bbox: 183,87,812,299

description: The diagram shows a multi-stage process starting with Image Patches.

[/visual_element]

**Figure 2: Pixtral Vision Encoder.**

[visual_element]

type: table

bbox: 566,413,816,556

title: Table 1: Decoder and encoder parameters.

description: The table presents the parameter counts.

html: <table><tr><td>dim</td><td>5120</td></tr></table>

[/visual_element]

### 2.2 Vision Encoder
`

func TestParseVisionGeneration(t *testing.T) {
	gen, err := ParseVisionGeneration(pixtralSample)
	if err != nil {
		t.Fatal(err)
	}
	if len(gen.Segments) != 4 {
		t.Fatalf("got %d segments, want 4: %+v", len(gen.Segments), gen.Segments)
	}
	if gen.Segments[0].Type != VisionSegmentTypeElement || gen.Segments[0].Element == nil {
		t.Fatalf("segment 0: %+v", gen.Segments[0])
	}
	if gen.Segments[0].Element.Type != "flowchart" {
		t.Fatalf("type=%q", gen.Segments[0].Element.Type)
	}
	if gen.Segments[0].Element.BBox == nil || gen.Segments[0].Element.BBox.TopLeftX != 183 {
		t.Fatalf("bbox=%+v", gen.Segments[0].Element.BBox)
	}
	if gen.Segments[1].Type != VisionSegmentTypeText || !strings.Contains(gen.Segments[1].Text, "Figure 2") {
		t.Fatalf("segment 1: %+v", gen.Segments[1])
	}
	if gen.Segments[2].Element == nil || gen.Segments[2].Element.Type != "table" {
		t.Fatalf("segment 2: %+v", gen.Segments[2])
	}
	if gen.Segments[2].Element.HTML == nil || !strings.Contains(*gen.Segments[2].Element.HTML, "<table>") {
		t.Fatalf("html=%v", gen.Segments[2].Element.HTML)
	}
}

func TestParseVisionGenerationUnclosed(t *testing.T) {
	_, err := ParseVisionGeneration("[visual_element]\ntype: table\n")
	if err == nil {
		t.Fatal("expected error")
	}
	if !strings.Contains(err.Error(), "unclosed") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestParseVisionGenerationProseOnly(t *testing.T) {
	gen, err := ParseVisionGeneration("hello\n\nworld\n")
	if err != nil {
		t.Fatal(err)
	}
	if len(gen.Segments) != 1 || gen.Segments[0].Type != VisionSegmentTypeText {
		t.Fatalf("%+v", gen)
	}
}
