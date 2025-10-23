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
	"reflect"
)

// Mapping_Items returns a list of the items in object o, where each item
// is a tuple containing a key-value pair.
//
// C API: https://docs.python.org/3/c-api/mapping.html#c.PyMapping_Items
func (Py Py) Mapping_Items(o Object) (Object, error) {
	return Py.wrap(C.PyMapping_Items(o.o))
}

// mappingFromObject converts a Python object supporting the mapping
// protocol to a Go map value.
func mappingFromObject(Py Py, o Object, a interface{}) error {
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

	o_items, err := Py.Mapping_Items(o)
	defer Py.DecRef(o_items)
	if err != nil {
		return err
	}

	length, err := Py.Object_Length(o_items)
	if err != nil {
		return err
	}
	for i := 0; i < length; i++ {
		o_item, err := Py.Sequence_GetItem(o_items, i)
		defer Py.DecRef(o_item)
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
	c := GoConvConf{
		Kind:       reflect.Map,
		FromObject: mappingFromObject,
	}
	if err := GoRegisterConversions(c); err != nil {
		panic(err)
	}
}
