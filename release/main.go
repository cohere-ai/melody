package main

import (
	"fmt"

	melody "github.com/cohere-ai/melody/release/go-bindings"
)
// main is a test function to ensure the release build is minimally
// functional (mostly that it's importable and callable)
func main() {
    filter := melody.NewFilter()
    fo := filter.WriteDecoded("Hello", nil)
    for _, output := range fo {
        fmt.Println(output.Text)
    }
}
