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
// int pyList_CheckExact(PyObject *o)
// {
//     return PyList_CheckExact(o);
// }
import "C"
import (
	"fmt"
	"reflect"
)

// List_Type wraps the Python PyList_Type type object.
//
// C API: https://docs.python.org/3/c-api/list.html#c.PyList_Type
var List_Type = TypeObject{&C.PyList_Type}

// List_CheckExact returns true if o is of type list, subtypes excluded.
//
// C API: https://docs.python.org/3/c-api/list.html#c.PyList_CheckExact
func (Py Py) List_CheckExact(o Object) bool {
	return C.pyList_CheckExact(o.o) != 0
}

// List_New returns a new list object of size length.
//
// C API: https://docs.python.org/3/c-api/list.html#c.PyList_New
func (Py Py) List_New(length int) (Object, error) {
	return Py.wrap(C.PyList_New(C.Py_ssize_t(length)))
}

// List_GetItem returns the object at position pos in the list o.
//
// C API: https://docs.python.org/3/c-api/list.html#c.PyList_GetItem
func (Py Py) List_GetItem(o Object, pos int) (Object, error) {
	return Py.wrap(C.PyList_GetItem(o.o, C.Py_ssize_t(pos)))
}

// List_SetItem inserts a reference to item at position pos of the list o.
//
// If item is of type Object it is used as-is, otherwise it's first
// converted with GoToObject.
//
// Note: this function “steals” a reference to item and discards a
// reference to an item already in the list at the affected position.
//
// C API: https://docs.python.org/3/c-api/list.html#c.PyList_SetItem
func (Py Py) List_SetItem(o Object, pos int, item interface{}) error {
	o_item, err := Py.GoToObject(item)
	if err != nil {
		return err
	}
	if C.PyList_SetItem(o.o, C.Py_ssize_t(pos), o_item.o) != 0 {
		Py.DecRef(o_item)
		return Py.GoCatchError()
	}
	if _, ok := item.(Object); ok {
		// Passed Objects get their refcount incremented by Py.GoToObject, we need to
		// DecRef them to comply with the "reference stealing" semantics of this function.
		Py.DecRef(o_item)
	}
	return nil
}

// List_Append appends item at the end of list o.
//
// C API: https://docs.python.org/3/c-api/list.html#c.PyList_Append
func (Py Py) List_Append(o Object, item interface{}) error {
	o_item, err := Py.GoToObject(item)
	defer Py.DecRef(o_item)
	if err != nil {
		return err
	}
	if C.PyList_Append(o.o, o_item.o) != 0 {
		return Py.GoCatchError()
	}
	return nil
}

// List_Insert inserts item at position pos of the list o.
//
// C API: https://docs.python.org/3/c-api/list.html#c.PyList_Insert
func (Py Py) List_Insert(o Object, pos int, item interface{}) error {
	o_item, err := Py.GoToObject(item)
	defer Py.DecRef(o_item)
	if err != nil {
		return err
	}
	if C.PyList_Insert(o.o, C.Py_ssize_t(pos), o_item.o) != 0 {
		return Py.GoCatchError()
	}
	return nil
}

// listToObject converts a Go slice value to a Python list.
func listToObject(Py Py, a interface{}) (Object, error) {
	v := reflect.ValueOf(a)
	o_list, err := Py.List_New(v.Len())
	if err != nil {
		return Object{}, err
	}
	for i := 0; i < v.Len(); i++ {
		a_item := v.Index(i).Interface()
		err = Py.List_SetItem(o_list, i, a_item)
		if err != nil {
			Py.DecRef(o_list)
			return Object{}, err
		}
		if o_item, ok := a_item.(Object); ok {
			// List_SetItem has the "reference stealing" semantics but ToObject
			// has not. Let's adjust the refcount for passed Objects.
			Py.IncRef(o_item)
		}
	}
	return o_list, nil
}

// listFromObject converts a Python list to a Go slice value.
func listFromObject(Py Py, o Object, a interface{}) error {
	length, err := Py.Object_Length(o)
	if err != nil {
		return err
	}
	slice := reflect.ValueOf(a).Elem()
	switch slice.Kind() {
	case reflect.Slice:
		if slice.IsNil() {
			slice = reflect.MakeSlice(slice.Type(), 0, length)
		}
	case reflect.Interface:
		if slice.IsNil() {
			slice = reflect.ValueOf(make([]interface{}, 0, length))
		}
	default:
		return Py.GoErrorConvFromObject(o, a)
	}
	elem_type := slice.Type().Elem()
	for i := 0; i < length; i++ {
		o_item, err := Py.List_GetItem(o, i)
		if err != nil {
			return err
		}
		elem := reflect.New(elem_type)
		err = Py.GoFromObject(o_item, elem.Interface())
		if err != nil {
			return fmt.Errorf("item #%d: %s", i, err)
		}
		slice = reflect.Append(slice, reflect.Indirect(elem))
	}
	reflect.ValueOf(a).Elem().Set(slice)
	return nil
}

func init() {
	cc := []GoConvConf{
		{
			Kind:     reflect.Slice,
			ToObject: listToObject,
		}, {
			TypeObject: List_Type,
			FromObject: listFromObject,
		},
	}
	if err := GoRegisterConversions(cc...); err != nil {
		panic(err)
	}
}
