# Golang Melody

## Parsing

```Go
import "github.com/cohere-ai/melody/parsing"
textChunks := []string{
    "<|START_THINKING|>", "This", " is", " a", " rainbow", " <", "co", ">", "emoji", ":", " 🌈",
    "</", "co", ":", " ", "0", ":[", "1", "]>", "<|END_THINKING|>", "\n", "<|START_RESPONSE|>",
    "foo", " <", "co", ">", "bar", "</", "co", ":", " ", "0", ":[", "1", ",", "2", "],", "1",
    ":[", "3", ",", "4", "]>", "<|END_RESPONSE|>"
}
f := parsing.NewFilter(logger, nil, parsing.HandleMultiHopCmd3(), parsing.StreamToolActions())
var wg sync.WaitGroup
defer wg.Wait()
wg.Go(func(){
    defer f.Close()
    for _, chunk := range(textChunks) {
        f.WriteDecoded(token, nil)
    }
})
out := []FilterOutput{}
for o := range f.Read() {
    out = append(out, o...)
} // FlushPartials called internally when f is closed

/*
[]FilterOutput{
    {IsToolsReason: true, Text: "This"},
    {IsToolsReason: true, Text: " is"},
    {IsToolsReason: true, Text: " a"},
    {IsToolsReason: true, Text: " rainbow"},
    {IsToolsReason: true, Text: " "},
    {IsToolsReason: true, Text: "emoji"},
    {IsToolsReason: true, Text: ":"},
    {IsToolsReason: true, Text: " 🌈"},
    {IsToolsReason: true, Citations: []FilterCitation{{
        StartIndex: 18,
        EndIndex:   26,
        Text:       "emoji: 🌈",
        Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1}}},
        IsThinking: true,
    }}},
    {Text: "foo"},
    {Text: " "},
    {Text: "bar"},
    {Citations: []FilterCitation{{
        StartIndex: 4,
        EndIndex:   7,
        Text:       "bar",
        Sources:    []Source{{ToolCallIndex: 0, ToolResultIndices: []int{1, 2}}, {ToolCallIndex: 1, ToolResultIndices: []int{3, 4}}},
        IsThinking: false,
    }}},
},
 */
```
