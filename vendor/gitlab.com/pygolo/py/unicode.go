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
//
// int pyUnicode_CheckExact(PyObject *o)
// {
//     return PyUnicode_CheckExact(o);
// }
import "C"
import (
	"reflect"
	"unsafe"
)

// Unicode_Type wraps the Python PyUnicode_Type type object.
//
// C API: https://docs.python.org/3/c-api/unicode.html#c.PyUnicode_Type
var Unicode_Type = TypeObject{&C.PyUnicode_Type}

// Unicode_CheckExact returns true if o is of type Unicode, subtypes excluded.
//
// C API: https://docs.python.org/3/c-api/unicode.html#c.PyUnicode_CheckExact
func (Py Py) Unicode_CheckExact(o Object) bool {
	return C.pyUnicode_CheckExact(o.o) != 0
}

// Unicode_FromString creates a Unicode object from a UTF-8 encoded string.
//
// C API: https://docs.python.org/3/c-api/unicode.html#c.PyUnicode_FromString
func (Py Py) Unicode_FromString(arg string) (Object, error) {
	c_arg := C.CString(arg)
	c_arg_size := C.Py_ssize_t(len(arg))
	defer C.free(unsafe.Pointer(c_arg))
	return Py.wrap(C.PyUnicode_FromStringAndSize(c_arg, c_arg_size))
}

// Unicode_AsUTF8 returns the UTF-8 encoding of the Unicode object.
//
// C API: https://docs.python.org/3/c-api/unicode.html#c.PyUnicode_AsUTF8AndSize
func (Py Py) Unicode_AsUTF8(o Object) (string, error) {
	var size C.Py_ssize_t
	c_s := C.PyUnicode_AsUTF8AndSize(o.o, &size)
	if c_s == nil {
		return "", Py.GoCatchError()
	}
	return C.GoStringN(c_s, C.int(size)), nil
}

// Unicode_GetLength returns the length of the Unicode object, in runes.
//
// C API: https://docs.python.org/3/c-api/unicode.html#c.PyUnicode_GetLength
func (Py Py) Unicode_GetLength(o Object) int {
	return int(C.PyUnicode_GetLength(o.o))
}

// Unicode_DecodeFSDefault decodes a string from the filesystem encoding
// and error handler.
//
// C API: https://docs.python.org/3/c-api/unicode.html#c.PyUnicode_DecodeFSDefault
func (Py Py) Unicode_DecodeFSDefault(arg string) (Object, error) {
	c_arg := C.CString(arg)
	c_arg_size := C.Py_ssize_t(len(arg))
	defer C.free(unsafe.Pointer(c_arg))
	return Py.wrap(C.PyUnicode_DecodeFSDefaultAndSize(c_arg, c_arg_size))
}

// unicodeToObject converts a Go string value to a Python unicode object.
func unicodeToObject(Py Py, a interface{}) (Object, error) {
	return Py.Unicode_FromString(a.(string))
}

// unicodeFromObject converts a Python unicode object to a Go string value.
func unicodeFromObject(Py Py, o Object, a interface{}) error {
	var value string
	var err error

	var o_str Object
	defer Py.DecRef(o_str)

	switch target := a.(type) {
	case *string:
		if o_str, err = Py.Object_Str(o); err == nil {
			if value, err = Py.Unicode_AsUTF8(o_str); err == nil {
				*target = value
			}
		}
	case *interface{}:
		if o_str, err = Py.Object_Str(o); err == nil {
			if value, err = Py.Unicode_AsUTF8(o_str); err == nil {
				*target = value
			}
		}
	default:
		err = Py.GoErrorConvFromObject(o, a)
	}
	return err
}

func init() {
	c := GoConvConf{
		Kind:       reflect.String,
		TypeObject: Unicode_Type,
		ToObject:   unicodeToObject,
		FromObject: unicodeFromObject,
	}
	if err := GoRegisterConversions(c); err != nil {
		panic(err)
	}
}
