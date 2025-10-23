/*
 * Copyright 2022, Pygolo Project contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package py

import "os"

// GoFile describes how a Go os.File is exposed to Python.
//
// The file is effectively re-opened by the Python interpreter
// by using the `open` function, Mode and Encoding are passed
// as parameters to that `open` invocation.
//
// The internal OS representation of the file is shared between
// the Go os.File value and the Python file object, therefore
// changes of the current position within the file are reflected
// across the Go-Python boundarie.
//
// When the two runtimes concurrently update the file you need
// to properly handle the buffering logic of each.
//
// The lifetime of the two representations is independent, each
// runtime owns its own instance; the file is effectively closed
// only when both the runtimes close it.
//
// Example:
//
//	func Write(Py py.Py, filename string) {
//	    f, err := os.Create(filename)
//
//	    defer func() {
//	        err := f.Close()
//	    }()
//
//	    of, err := Py.GoToObject(py.GoFile{File: f, Mode: "w"})
//
//	    defer Py.DecRef(of)
//	    defer func() {
//	        o_ret, err := Py.Object_CallMethod(of, "close")
//	        Py.DecRef(o_ret)
//	    }()
//
//	    f.WriteString("Written from Go\n")
//	    f.Sync()
//
//	    o_ret, err := Py.Object_CallMethod(of, "write", "Written from Python\n")
//	    Py.DecRef(o_ret)
//	}
type GoFile struct {
	// File holds the pointer to the os.File being exposed.
	File *os.File

	// Mode specifies the mode in which the file is opened. Default is "r".
	Mode string

	// Encoding is the encoding used to decode or encode the text file. Default is "utf-8".
	Encoding string
}

func init() {
	cc := []GoConvConf{
		{
			TypeOf:     (*os.File)(nil),
			ToObject:   fileToObject,
			FromObject: fileFromObject,
		}, {
			TypeOf:   GoFile{},
			ToObject: fileToObject,
		},
	}
	if err := GoRegisterConversions(cc...); err != nil {
		panic(err)
	}
}
