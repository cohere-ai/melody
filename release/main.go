package main

import (
	"fmt"

	melody "github.com/cohere-ai/melody/release/go-bindings"
)

func main() {
    filter := melody.NewFilter()
    fo := filter.WriteDecoded("Hello", nil)
    for _, output := range fo {
        fmt.Println(output.Text)
    }
}