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
// PyObject* py_CompileString(const char *str, const char *filename, int start)
// {
//     return Py_CompileString(str, filename, start);
// }
import "C"
import "unsafe"

// Eval_input is the start symbol for isolated expressions.
//
// C API: https://docs.python.org/3/c-api/veryhigh.html#c.Py_eval_input
const Eval_input = int(C.Py_eval_input)

// File_input is the start symbol for arbitrarily long source code.
//
// C API: https://docs.python.org/3/c-api/veryhigh.html#c.Py_file_input
const File_input = int(C.Py_file_input)

// Single_input is the start symbol for single statement.
//
// C API: https://docs.python.org/3/c-api/veryhigh.html#c.Py_single_input
const Single_input = int(C.Py_single_input)

// CompileString parses and compiles the Python source code in str,
// returns the resulting code object.
//
// C API: https://docs.python.org/3/c-api/veryhigh.html#c.Py_CompileString
func (Py Py) CompileString(str, filename string, start int) (Object, error) {
	c_str := C.CString(str)
	defer C.free(unsafe.Pointer(c_str))
	c_filename := C.CString(filename)
	defer C.free(unsafe.Pointer(c_filename))
	return Py.wrap(C.py_CompileString(c_str, c_filename, C.int(start)))
}

// Eval_EvalCode evaluates a precompiled code object.
//
// C API: https://docs.python.org/3/c-api/veryhigh.html#c.PyEval_EvalCode
func (Py Py) Eval_EvalCode(code, globals, locals Object) (Object, error) {
	return Py.wrap(C.PyEval_EvalCode(code.o, globals.o, locals.o))
}

// Eval_GetBuiltins returns a dictionary of the builtins in the current
// execution frame, or the interpreter of the thread state if no frame is
// currently executing.
//
// C API: https://docs.python.org/3/c-api/reflection.html#c.PyEval_GetBuiltins
func (Py Py) Eval_GetBuiltins() (Object, error) {
	return Py.wrap(C.PyEval_GetBuiltins())
}
