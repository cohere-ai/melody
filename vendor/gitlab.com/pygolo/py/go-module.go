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
// #include "go-module.h"
import "C"
import (
	"fmt"
	"unsafe"
)

// GoModule_Init is the type of a module initialization function.
//
// Py is the interpreter context, Object is the module object.
type GoModule_Init func(Py, Object) error

// GoExtend returns a module definition ready for the interpreter.
//
// The interpreter will execute the init function to complete
// the module initialization.
func GoExtend(init GoModule_Init) unsafe.Pointer {
	def := C.pgl_new_moduledef(C.Py_ssize_t(unsafe.Sizeof(GoHandle{})))
	if def == nil {
		return unsafe.Pointer(C.PyErr_NoMemory())
	}
	o_def := C.PyModuleDef_Init(def)
	if o_def == nil {
		C.pgl_del_moduledef(def)
		return nil
	}
	(*GoHandle)(C.pgl_get_moduledef_data(def)).Set(init)
	return unsafe.Pointer(o_def)
}

//export delModule
func delModule(o *C.PyObject) {
	m := Object{o}
	Py := extendModule(m)
	Py.deinitTypes()
	Py.Close()
	def := C.PyModule_GetDef(m.o)
	(*GoHandle)(C.pgl_get_moduledef_data(def)).Close()
	C.pgl_del_moduledef(def)
}

//export initModule
func initModule(o *C.PyObject) C.int {
	m := Object{o}
	Py := extendModule(m)
	Py.state.Set(&state{})
	err := Py.initTypes(m)
	if err != nil {
		Py.GoSetError(err)
		return -1
	}
	def := C.PyModule_GetDef(m.o)
	handle := (*GoHandle)(C.pgl_get_moduledef_data(def))
	init := handle.Get().(GoModule_Init)
	if init != nil {
		err = safeInitModule(init, Py, m)
	}
	if err != nil {
		Py.GoSetError(err)
		return -1
	}
	return 0
}

func safeInitModule(init GoModule_Init, Py Py, m Object) (err error) {
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("panic: %v", r)
		}
	}()
	return init(Py, m)
}

func (Py Py) moduleAddType(m Object, t TypeObject) error {
	m_name, err := Py.wrap(C.PyModule_GetNameObject(m.o))
	defer Py.DecRef(m_name)
	if err != nil {
		return err
	}
	t_dict, err := Py.wrap(C.PyObject_GenericGetDict(t.AsObject().o, nil))
	defer Py.DecRef(t_dict)
	if err != nil {
		return err
	}
	err = Py.Dict_SetItem(t_dict, "__module__", m_name)
	if err != nil {
		return err
	}
	err = Py.Object_SetAttr(m, t.Name(), t)
	if err != nil {
		return err
	}
	return nil
}
