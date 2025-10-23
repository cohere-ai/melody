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

// #include "pygolo.h"
import "C"
import (
	"fmt"
	"path/filepath"
	"runtime"
	"strings"
)

// wrap either wraps a C API object or fetches an error if it's nil.
func (Py Py) wrap(o *C.PyObject) (Object, error) {
	if o == nil {
		return Object{}, Py.GoCatchError()
	}
	return Object{o}, nil
}

// GoError corresponds to a Python exception.
//
// As defined by the C API, such error indicator is composed by
// exception type, value and traceback objects populated by PyErr_Fetch.
//
// The proper way to retrieve an GoError instance is by using GoCatchError.
type GoError struct {
	// Type is the name of the exception type.
	Type string
	// Value is the text representation of the exception.
	Value string
	// Traceback is the text representation of the call stack at the
	// point where the exception last occurred.
	Traceback []string
}

// GoError combines Type, Value and Traceback as they would be printed by
// the interpreter.
func (e *GoError) Error() string {
	return fmt.Sprintf("%s%s: %s", strings.Join(e.Traceback, ""), e.Type, e.Value)
}

// GoCatchError fetches an exception and returns it as an error.
//
// If no exception is pending, nil is returned.
func (Py Py) GoCatchError() error {
	var o_type, o_value, o_tb Object
	Py.Err_Fetch(&o_type, &o_value, &o_tb)
	if o_type.o == nil && o_value.o == nil && o_tb.o == nil {
		return nil
	}
	Py.Err_NormalizeException(&o_type, &o_value, &o_tb)
	defer Py.DecRef(o_type, o_value, o_tb)
	if o_tb.o == nil {
		o_tb = None
	}
	var s_value string
	err := Py.GoFromObject(o_value, &s_value)
	if err != nil {
		return err
	}
	o_traceback, err := Py.Import_Import("traceback")
	defer Py.DecRef(o_traceback)
	if err != nil {
		return err
	}
	o_tb_lines, err := Py.Object_CallMethod(o_traceback, "format_exception", o_type, o_value, o_tb)
	defer Py.DecRef(o_tb_lines)
	if err != nil {
		return err
	}
	var s_tb_lines []string
	err = Py.GoFromObject(o_tb_lines, &s_tb_lines)
	if err != nil {
		return err
	}
	// either the stack trace only or nothing
	if len(s_tb_lines) > 1 {
		s_tb_lines = s_tb_lines[:len(s_tb_lines)-1]
	} else {
		s_tb_lines = nil
	}
	if s_tb_lines != nil {
		pc := make([]uintptr, 64)
		num := runtime.Callers(2, pc)
		// reverse the callers, Python and Go present them in the opposite order
		for i := 0; i < num/2; i++ {
			j := num - i - 1
			pc[i], pc[j] = pc[j], pc[i]
		}
		frames := runtime.CallersFrames(pc[:num])
		go_tb_lines := make([]string, 0, num+len(s_tb_lines))
		// first, the Python traceback header
		go_tb_lines = append(go_tb_lines, s_tb_lines[0])
		// second, the Go callers frames
		for more := true; more; {
			var frame runtime.Frame
			frame, more = frames.Next()
			frameFile := filepath.FromSlash(frame.File)
			// ignore the initial part of the stack that is rooted in the runtime
			if strings.HasPrefix(frameFile, runtime.GOROOT()) && len(go_tb_lines) == 1 {
				continue
			}
			// skip the last call to Py.wrap
			if !more && frame.Function == "gitlab.com/pygolo/py.Py.wrap" {
				continue
			}
			line := fmt.Sprintf("  File \"%s\", line %d, in %s\n", frameFile, frame.Line, frame.Function)
			go_tb_lines = append(go_tb_lines, line)
		}
		// third, the Python stack trace
		s_tb_lines = append(go_tb_lines, s_tb_lines[1:]...)
	}
	s_type := o_value.Type().Name()
	return &GoError{s_type, s_value, s_tb_lines}
}

// GoSetError sets the Python error indicator in base of err.
func (Py Py) GoSetError(err error) {
	if pyerr, ok := err.(*GoError); !ok {
		Py.Err_SetString(Exc_RuntimeError, err.Error())
	} else if exc, ok := exceptionsByName[pyerr.Type]; !ok {
		Py.Err_SetString(Exc_RuntimeError, pyerr.Value)
	} else {
		Py.Err_SetString(exc, pyerr.Value)
	}
}
