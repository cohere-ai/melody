package main

import (
	"C"
	"unsafe"

	"gitlab.com/pygolo/py"

	"github.com/cohere-ai/melody"
)

func testFunc(m melody.Message) string {
	return m.Role.String()
}

func myext(Py py.Py, m py.Object) error {
	// here goes the extension initialization code
	if err := Py.Module_SetDocString(m, "Melody provides templating and parsing for Cohere models."); err != nil {
		return err
	}
	err := Py.GoRegisterStruct(melody.Message{})
	if err != nil {
		return nil
	}
	if err := Py.Object_SetAttr(m, "TestFunc", testFunc); err != nil {
		return err
	}

	return nil
}

//export PyInit_melody
func PyInit_melody() unsafe.Pointer {
	return py.GoExtend(myext)
}

// required by cgo but unused
func main() {
}
