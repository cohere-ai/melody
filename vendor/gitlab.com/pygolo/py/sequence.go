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

// Sequence_GetItem returns the object at position pos in the sequence o.
//
// C API: https://docs.python.org/3/c-api/sequence.html#c.PySequence_GetItem
func (Py Py) Sequence_GetItem(o Object, pos int) (Object, error) {
	return Py.wrap(C.PySequence_GetItem(o.o, C.Py_ssize_t(pos)))
}

// sequenceFromObject converts a Python object supporting the sequence
// protocol to a Go slice value.
func sequenceFromObject(Py Py, o Object, a interface{}) error {
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
		o_item, err := Py.Sequence_GetItem(o, i)
		if err != nil {
			return err
		}
		elem := reflect.New(elem_type)
		err = Py.GoFromObject(o_item, elem.Interface())
		Py.DecRef(o_item)
		if err != nil {
			return fmt.Errorf("item #%d: %s", i, err)
		}
		slice = reflect.Append(slice, reflect.Indirect(elem))
	}
	reflect.ValueOf(a).Elem().Set(slice)
	return nil
}

func init() {
	c := GoConvConf{
		Kind:       reflect.Slice,
		FromObject: sequenceFromObject,
	}
	if err := GoRegisterConversions(c); err != nil {
		panic(err)
	}
}
