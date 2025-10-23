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
// int pyDict_CheckExact(PyObject *o)
// {
//     return PyDict_CheckExact(o);
// }
import "C"
import (
	"fmt"
	"reflect"
)

// Dict_Type wraps the Python PyDict_Type type object.
//
// C API: https://docs.python.org/3/c-api/dict.html#c.PyDict_Type
var Dict_Type = TypeObject{&C.PyDict_Type}

// Dict_CheckExact returns true if o is of type dict, subtypes excluded.
//
// C API: https://docs.python.org/3/c-api/dict.html#c.PyDict_CheckExact
func (Py Py) Dict_CheckExact(o Object) bool {
	return C.pyDict_CheckExact(o.o) != 0
}

// Dict_New returns a new empty dictionary.
//
// C API: https://docs.python.org/3/c-api/dict.html#c.PyDict_New
func (Py Py) Dict_New() (Object, error) {
	return Py.wrap(C.PyDict_New())
}

// Dict_Items returns a list of the items in object o, where each item
// is a tuple containing a key-value pair.
//
// C API: https://docs.python.org/3/c-api/dict.html#c.PyDict_Items
func (Py Py) Dict_Items(o Object) (Object, error) {
	return Py.wrap(C.PyDict_Items(o.o))
}

// Dict_GetItem returns the object from dictionary o which has a key key.
//
// C API: https://docs.python.org/3/c-api/dict.html#c.PyDict_GetItem
func (Py Py) Dict_GetItem(o Object, key interface{}) (Object, error) {
	o_key, err := Py.GoToObject(key)
	defer Py.DecRef(o_key)
	if err != nil {
		return Object{}, err
	}
	return Py.wrap(C.PyDict_GetItem(o.o, o_key.o))
}

// Dict_SetItem inserts value into the dictionary o with a key of key.
//
// C API: https://docs.python.org/3/c-api/dict.html#c.PyDict_SetItem
func (Py Py) Dict_SetItem(o Object, key, value interface{}) error {
	o_key, err := Py.GoToObject(key)
	defer Py.DecRef(o_key)
	if err != nil {
		return nil
	}
	o_value, err := Py.GoToObject(value)
	defer Py.DecRef(o_value)
	if err != nil {
		return nil
	}
	if C.PyDict_SetItem(o.o, o_key.o, o_value.o) != 0 {
		return Py.GoCatchError()
	}
	return nil
}

// dictToObject converts a Go map to a Python dictionary object.
func dictToObject(Py Py, a interface{}) (Object, error) {
	o_dict, err := Py.Dict_New()
	if err != nil {
		return Object{}, err
	}
	v := reflect.ValueOf(a)
	for _, key := range v.MapKeys() {
		err = Py.Dict_SetItem(o_dict, key.Interface(), v.MapIndex(key).Interface())
		if err != nil {
			Py.DecRef(o_dict)
			return Object{}, err
		}
	}
	return o_dict, nil
}

// dictFromObject converts a Python dictionary to a Go map.
func dictFromObject(Py Py, o Object, a interface{}) error {
	m := reflect.ValueOf(a).Elem()
	switch m.Kind() {
	case reflect.Map:
		if m.IsNil() {
			m = reflect.MakeMap(m.Type())
		}
	case reflect.Interface:
		if m.IsNil() {
			m = reflect.ValueOf(make(map[interface{}]interface{}))
		}
	default:
		return Py.GoErrorConvFromObject(o, a)
	}
	key_type := m.Type().Key()
	value_type := m.Type().Elem()

	o_items, err := Py.Dict_Items(o)
	defer Py.DecRef(o_items)
	if err != nil {
		return err
	}

	length, err := Py.Object_Length(o_items)
	if err != nil {
		return err
	}
	for i := 0; i < length; i++ {
		o_item, err := Py.List_GetItem(o_items, i)
		if err != nil {
			return err
		}
		o_key, err := Py.Tuple_GetItem(o_item, 0)
		if err != nil {
			return err
		}
		o_value, err := Py.Tuple_GetItem(o_item, 1)
		if err != nil {
			return err
		}
		key := reflect.New(key_type)
		if err := Py.GoFromObject(o_key, key.Interface()); err != nil {
			var str string
			if e := Py.GoFromObject(o_key, &str); e == nil {
				return fmt.Errorf("key %#v: %s", str, err)
			}
			return fmt.Errorf("key %#v: %s", o_key, err)
		}
		value := reflect.New(value_type)
		if err := Py.GoFromObject(o_value, value.Interface()); err != nil {
			var str string
			if e := Py.GoFromObject(o_value, &str); e == nil {
				return fmt.Errorf("value %#v: %s", str, err)
			}
			return fmt.Errorf("value %#v: %s", o_value, err)
		}
		m.SetMapIndex(reflect.Indirect(key), reflect.Indirect(value))
	}

	reflect.ValueOf(a).Elem().Set(m)
	return nil
}

func init() {
	cc := []GoConvConf{
		{
			Kind:     reflect.Map,
			ToObject: dictToObject,
		}, {
			TypeObject: Dict_Type,
			FromObject: dictFromObject,
		},
	}
	if err := GoRegisterConversions(cc...); err != nil {
		panic(err)
	}
}
