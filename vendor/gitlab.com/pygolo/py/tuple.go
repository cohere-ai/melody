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
// int pyTuple_CheckExact(PyObject *o)
// {
//     return PyTuple_CheckExact(o);
// }
import "C"
import (
	"fmt"
	"reflect"
)

// Tuple_Type wraps the Python PyTuple_Type type object.
//
// C API: https://docs.python.org/3/c-api/tuple.html#c.PyTuple_Type
var Tuple_Type = TypeObject{&C.PyTuple_Type}

// Tuple_CheckExact returns true if o is of type tuple, subtypes excluded.
//
// C API: https://docs.python.org/3/c-api/tuple.html#c.PyTuple_CheckExact
func (Py Py) Tuple_CheckExact(o Object) bool {
	return C.pyTuple_CheckExact(o.o) != 0
}

// Tuple_GetItem returns the object at position pos in the tuple o.
//
// C API: https://docs.python.org/3/c-api/tuple.html#c.PyTuple_GetItem
func (Py Py) Tuple_GetItem(o Object, pos int) (Object, error) {
	return Py.wrap(C.PyTuple_GetItem(o.o, C.Py_ssize_t(pos)))
}

// Tuple_Pack returns a new tuple initialized with items.
//
// Items of type Object are used as-is, others are first converted
// with GoToObject.
//
// C API: https://docs.python.org/3/c-api/tuple.html#c.PyTuple_Pack
func (Py Py) Tuple_Pack(items ...interface{}) (Object, error) {
	return tupleToObject(Py, items)
}

// tupleToObject converts a Go array value to a Python tuple.
func tupleToObject(Py Py, a interface{}) (Object, error) {
	v := reflect.ValueOf(a)
	o_tuple, err := Py.wrap(C.PyTuple_New(C.Py_ssize_t(v.Len())))
	if err != nil {
		return Object{}, err
	}
	for i := 0; i < v.Len(); i++ {
		o_item, err := Py.GoToObject(v.Index(i).Interface())
		if err != nil {
			Py.DecRef(o_tuple)
			return Object{}, err
		}
		if C.PyTuple_SetItem(o_tuple.o, C.Py_ssize_t(i), o_item.o) == -1 {
			Py.DecRef(o_tuple)
			Py.DecRef(o_item)
			return Object{}, Py.GoCatchError()
		}
	}
	return o_tuple, nil
}

// tupleFromObject converts a Python tuple to a Go slice value.
func tupleFromObject(Py Py, o Object, a interface{}) error {
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
		o_item, err := Py.Tuple_GetItem(o, i)
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
			Kind:     reflect.Array,
			ToObject: tupleToObject,
		}, {
			TypeObject: Tuple_Type,
			FromObject: tupleFromObject,
		},
	}
	if err := GoRegisterConversions(cc...); err != nil {
		panic(err)
	}
}
