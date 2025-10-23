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

// Module_SetDocString sets the docstring of module m to doc.
//
// C API: https://docs.python.org/3/c-api/module.html#c.PyModule_SetDocString
func (Py Py) Module_SetDocString(m Object, doc string) error {
	s_doc := C.CString(doc)
	if s_doc == nil {
		return fmt.Errorf("cannot allocate module docstring")
	}
	defer C.free(unsafe.Pointer(s_doc))
	if C.PyModule_SetDocString(m.o, s_doc) == -1 {
		return Py.GoCatchError()
	}
	return nil
}
