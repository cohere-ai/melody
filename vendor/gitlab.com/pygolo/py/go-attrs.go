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
	"unsafe"
)

// GoAttrs provides the Python get/set attributes protocol of a Go value.
type GoAttrs interface {
	// PyGetAttr looks up the given attribute name.
	//
	// A new reference of the attribute value is returned. If the
	// value is not a Python object a conversion is attempted.
	//
	// Access to the interpreter is allowed during this call.
	PyGetAttr(Py, string) (Object, error)

	// PySetAttr sets the value of the given attribute.
	//
	// If the new value is a Python object, its reference count is
	// incremented. If the value being overwritten is a Python object,
	// its reference count is decremented.
	//
	// Access to the interpreter is allowed during this call.
	PySetAttr(Py, string, interface{}) error
}

// GoAttrsMap implements the GoAttrs interface on top of a map.
type GoAttrsMap map[string]interface{}

// PyGetAttr looks up the given attribute name in the underlying map.
func (m GoAttrsMap) PyGetAttr(Py Py, attr_name string) (Object, error) {
	attr_value := m[attr_name]
	if attr_value == nil {
		return Py.NewRef(None), nil
	}
	return Py.GoToObject(attr_value)
}

// PySetAttr sets the value of the given attribute in the underlying map.
func (m GoAttrsMap) PySetAttr(Py Py, attr_name string, attr_value interface{}) error {
	if new_, ok := attr_value.(Object); ok {
		if new_.o == nil {
			attr_value = nil
		} else {
			Py.IncRef(new_)
		}
	}
	attr_value, m[attr_name] = m[attr_name], attr_value
	if old_, ok := attr_value.(Object); ok {
		Py.DecRef(old_)
	}
	return nil
}

//export pgl_get_attr
func pgl_get_attr(o_ *C.PyObject, attr_name_ unsafe.Pointer) (ret *C.PyObject) {
	Py, err := extendObject(Object{o_})
	if err != nil {
		Py.GoSetError(err)
		return nil
	}
	o, ok := getPygoloObjectHandle(o_).Get().(GoAttrs)
	if !ok {
		Py.Err_SetString(Exc_TypeError, "Go value doesn't implement GoAttrs")
		return nil
	}
	defer func() {
		if r := recover(); r != nil {
			Py.Err_Format(Exc_AttributeError, "panic: %v", r)
			ret = nil
		}
	}()
	attr_name := C.GoString((*C.char)(attr_name_))
	attr_value, err := o.PyGetAttr(Py, attr_name)
	if err != nil {
		Py.GoSetError(err)
		return nil
	}
	return attr_value.o
}

//export pgl_set_attr
func pgl_set_attr(o_, attr_value_ *C.PyObject, attr_name_ unsafe.Pointer) (ret C.int) {
	Py, err := extendObject(Object{o_})
	if err != nil {
		Py.GoSetError(err)
		return -1
	}
	o, ok := getPygoloObjectHandle(o_).Get().(GoAttrs)
	if !ok {
		Py.Err_SetString(Exc_TypeError, "Go value doesn't implement GoAttrs")
		return -1
	}
	defer func() {
		if r := recover(); r != nil {
			Py.Err_Format(Exc_AttributeError, "panic: %v", r)
			ret = -1
		}
	}()
	attr_name := C.GoString((*C.char)(attr_name_))
	attr_value := Object{attr_value_}
	err = o.PySetAttr(Py, attr_name, attr_value)
	if err != nil {
		Py.GoSetError(err)
		return -1
	}
	return 0
}
