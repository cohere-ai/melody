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
// #include "go-object.h"
import "C"
import "unsafe"

func newPygoloObject(t TypeObject, a interface{}) (Object, error) {
	// Allocate the Python object, add storage for the GoHandle
	o := C.pgl_new_object(t.t)
	if o == nil {
		return Object{}, &GoError{"MemoryError", "could not allocate PygoloObject", nil}
	}
	// Store the Go value in the Python object via GoHandle
	getPygoloObjectHandle(o).Set(a)
	return Object{o}, nil
}

//export delPygoloObject
func delPygoloObject(o *C.PyObject) {
	getPygoloObjectHandle(o).Close()
	C.pgl_del_object(o)
}

func getPygoloObjectHandle(o *C.PyObject) *GoHandle {
	return (*GoHandle)(C.pgl_get_object_data(o))
}

func (Py Py) pglCheck(o Object) bool {
	return Py.GoFunction_Check(o)
}

//export getPygoloObjectDataSize
func getPygoloObjectDataSize() uint {
	return uint(unsafe.Sizeof(GoHandle{}))
}
