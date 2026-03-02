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
var fullContent, fullReasoning string
var allCitations []melody.FilterCitation
for _, chunk := range textChunks {
    result, err := f.WriteDecoded(chunk)
    if err != nil {
        panic(err)
    }
    if result == nil {
        continue
    }
    if result.Content != nil {
        fullContent += *result.Content
    }
    if result.Reasoning != nil {
        fullReasoning += *result.Reasoning
    }
    allCitations = append(allCitations, result.Citations...)
}

// Flush any remaining partial outputs
flushResult, err := f.FlushPartials()
if err != nil {
    panic(err)
}
if flushResult != nil {
    if flushResult.Content != nil {
        fullContent += *flushResult.Content
    }
    if flushResult.Reasoning != nil {
        fullReasoning += *flushResult.Reasoning
    }
    allCitations = append(allCitations, flushResult.Citations...)
}

/*
Expected output:
fullContent = "foo bar"
fullReasoning = "This is a rainbow emoji: 🌈"
allCitations = []melody.FilterCitation{
    {StartIndex: 18, EndIndex: 26, Text: "emoji: 🌈",
     Sources: []melody.Source{{ToolCallIndex: 0, ToolResultIndices: []uint{1}}},
     IsThinking: true},
    {StartIndex: 4, EndIndex: 7, Text: "bar",
     Sources: []melody.Source{
         {ToolCallIndex: 0, ToolResultIndices: []uint{1, 2}},
         {ToolCallIndex: 1, ToolResultIndices: []uint{3, 4}},
     },
     IsThinking: false},
}
*/
```
