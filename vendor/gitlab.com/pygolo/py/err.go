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
	"unsafe"
)

// Err_Occurred tests whether the error indicator is set.
//
// You do not own a reference to the return value, so you do not
// need to DecRef it.
//
// C API: https://docs.python.org/3/c-api/exceptions.html#c.PyErr_Occurred
func (Py Py) Err_Occurred() Object {
	return Object{C.PyErr_Occurred()}
}

// Err_SetString sets the error indicator using an exception type and an
// error message.
//
// C API: https://docs.python.org/3/c-api/exceptions.html#c.PyErr_SetString
func (Py Py) Err_SetString(o Object, msg string) {
	c_msg := C.CString(msg)
	defer C.free(unsafe.Pointer(c_msg))
	C.PyErr_SetString(o.o, c_msg)
}

// Err_Format sets the error indicator using an exception type, a
// format string and related values.
//
// C API: https://docs.python.org/3/c-api/exceptions.html#c.PyErr_Format
func (Py Py) Err_Format(o Object, format string, a ...interface{}) {
	Py.Err_SetString(o, fmt.Sprintf(format, a...))
}

// Err_NoMemory sets a MemoryError with None value.
//
// C API: https://docs.python.org/3/c-api/exceptions.html#c.PyErr_NoMemory
func (Py Py) Err_NoMemory() Object {
	return Object{C.PyErr_NoMemory()}
}

// Err_Fetch retrieves the error indicator into o_type, o_value and o_tb.
//
// C API: https://docs.python.org/3/c-api/exceptions.html#c.PyErr_Fetch
func (Py Py) Err_Fetch(o_type, o_value, o_tb *Object) {
	C.PyErr_Fetch(&o_type.o, &o_value.o, &o_tb.o)
}

// Err_NormalizeException normalizes the values returned by Err_Fetch.
//
// C API: https://docs.python.org/3/c-api/exceptions.html#c.PyErr_NormalizeException
func (Py Py) Err_NormalizeException(o_type, o_value, o_tb *Object) {
	C.PyErr_NormalizeException(&o_type.o, &o_value.o, &o_tb.o)
}
