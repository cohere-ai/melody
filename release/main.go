package main

import (
	"fmt"

	melody "github.com/cohere-ai/melody/release/gobindings"
	"github.com/cohere-ai/melody/release/gobindings/tokenizers"
)
// main is a test function to ensure the release build is minimally
// functional (mostly that it's importable and callable). This gets run from
// the release Dockerfile before finishing the build to ensure it works.
func main() {

    // **********
    // TEMPLATING
    // **********



    // *******
    // PARSING
    // *******
    filter := melody.NewFilter()
    fo := filter.WriteDecoded("Hello", nil)
    for _, output := range fo {
        fmt.Println(output.Text)
    }

    // **********
    // TOKENIZERS
    // **********
    tk, err := tokenizers.FromFile("./gobindings/data/bert-base-uncased.json")
    if err != nil {
        panic(err)
    }
    // release native resources
    defer tk.Close()
    fmt.Println("Vocab size:", tk.VocabSize())
    // Vocab size: 30522
    fmt.Println(tk.Encode("brown fox jumps over the lazy dog", false))
    // [2829 4419 14523 2058 1996 13971 3899] [brown fox jumps over the lazy dog]
    fmt.Println(tk.Encode("brown fox jumps over the lazy dog", true))
    // [101 2829 4419 14523 2058 1996 13971 3899 102] [[CLS] brown fox jumps over the lazy dog [SEP]]
    fmt.Println(tk.Decode([]uint32{2829, 4419, 14523, 2058, 1996, 13971, 3899}, true))
    // brown fox jumps over the lazy dog
}
