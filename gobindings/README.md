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
    outputs, err := f.WriteDecoded(chunk, nil)
    if err != nil {
        panic(err)
    }
    out = append(out, outputs...)
}

// Flush any remaining partial outputs
remaining, err := f.FlushPartials()
if err != nil {
    panic(err)
}
out = append(out, remaining...)

/*
Expected output:
[]melody.FilterOutput{
    {IsReasoning: true, Text: "This"},
    {IsReasoning: true, Text: " is"},
    {IsReasoning: true, Text: " a"},
    {IsReasoning: true, Text: " rainbow"},
    {IsReasoning: true, Text: " "},
    {IsReasoning: true, Text: "emoji"},
    {IsReasoning: true, Text: ":"},
    {IsReasoning: true, Text: " 🌈"},
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
