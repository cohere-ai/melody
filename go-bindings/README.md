# Golang Melody

## Parsing

```Go
import "github.com/cohere-ai/melody"

textChunks := []string{
    "<|START_THINKING|>", "This", " is", " a", " rainbow", " <", "co", ">", "emoji", ":", " 🌈",
    "</", "co", ":", " ", "0", ":[", "1", "]>", "<|END_THINKING|>", "\n", "<|START_RESPONSE|>",
    "foo", " <", "co", ">", "bar", "</", "co", ":", " ", "0", ":[", "1", ",", "2", "],", "1",
    ":[", "3", ",", "4", "]>", "<|END_RESPONSE|>"
}

// Create a filter with options using the builder pattern
f := melody.NewFilter(melody.HandleMultiHopCmd3(), melody.StreamToolActions())

// Process tokens synchronously
out := []melody.FilterOutput{}
for _, chunk := range textChunks {
    outputs := f.WriteDecoded(chunk, nil)
    out = append(out, outputs...)
}

// Flush any remaining partial outputs
out = append(out, f.FlushPartials()...)

/*
Expected output:
[]melody.FilterOutput{
    {IsToolsReason: true, Text: "This"},
    {IsToolsReason: true, Text: " is"},
    {IsToolsReason: true, Text: " a"},
    {IsToolsReason: true, Text: " rainbow"},
    {IsToolsReason: true, Text: " "},
    {IsToolsReason: true, Text: "emoji"},
    {IsToolsReason: true, Text: ":"},
    {IsToolsReason: true, Text: " 🌈"},
    {Citations: []melody.FilterCitation{{
        StartIndex: 18,
        EndIndex:   26,
        Text:       "emoji: 🌈",
        Sources:    []melody.Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
        IsThinking: true,
    }}},
    {Text: "foo"},
    {Text: " "},
    {Text: "bar"},
    {Citations: []melody.FilterCitation{{
        StartIndex: 4,
        EndIndex:   7,
        Text:       "bar",
        Sources:    []melody.Source{{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}}, {ToolCallIndex: 1, ToolResultIndices: []int{3, 4}}},
        IsThinking: false,
    }}},
}
*/
```
