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

import (
	"fmt"
	"reflect"
	"strings"
)

type goStructState struct {
	types map[reflect.Type]TypeObject
}

// GoGetStructType returns the Python type corresponding to a.
//
// If a is not a struct or was not previously registered with
// Py.GoRegisterStruct, the returned boolean is false.
//
// The returned TypeObject is a new reference that needs to be
// disposed with Py.DecRef after use.
//
// Example:
//
//	func PrintTypeOf(Py Py, s SomeStruct) {
//		if t, ok := Py.GoGetStructType(s); ok {
//			fmt.Printf("t: %s\n", t.Name())
//			Py.DecRef(t.AsObject())
//		}
//	}
func (Py Py) GoGetStructType(a interface{}) (TypeObject, bool) {
	t, ok := Py.getStructType(a)
	Py.IncRef(t.AsObject())
	return t, ok
}

func (Py Py) getStructType(a interface{}) (TypeObject, bool) {
	state := Py.getState()
	t, ok := state.structs.types[reflect.TypeOf(a)]
	return t, ok
}

func (Py Py) addStructType(a interface{}, t TypeObject) {
	state := Py.getState()
	if state.structs.types == nil {
		state.structs.types = make(map[reflect.Type]TypeObject)
	}
	state.structs.types[reflect.TypeOf(a)] = t
}

func (Py Py) delStructType(a interface{}) {
	state := Py.getState()
	delete(state.structs.types, reflect.TypeOf(a))
}

func (Py Py) createStructType(name string, fields_name []string) (TypeObject, error) {
	o_collections, err := Py.Import_Import("collections")
	defer Py.DecRef(o_collections)
	if err != nil {
		return TypeObject{}, err
	}
	o_tuple, err := Py.Object_CallMethod(o_collections, "namedtuple", name, fields_name)
	if err != nil {
		return TypeObject{}, err
	}
	return o_tuple.AsTypeObject()
}

func getStructFields(a interface{}) ([]string, []int, error) {
	t := reflect.TypeOf(a)
	if t == nil || t.Kind() != reflect.Struct {
		return nil, nil, fmt.Errorf("not a struct: %T", a)
	}

	names := make([]string, 0)
	positions := make([]int, 0)

fields:
	for i := 0; i < t.NumField(); i++ {
		f := t.Field(i)
		// Skip field if not exported
		if f.PkgPath != "" {
			continue
		}
		tags := strings.Split(f.Tag.Get("python"), ",")
		var unknown []string
		for _, tag := range tags[1:] {
			// Skip field if to be omitted
			if tag == "omit" {
				continue fields
			}
			unknown = append(unknown, tag)
		}
		if len(unknown) > 0 {
			return nil, nil, fmt.Errorf("field '%s' has unknown tags: %v", f.Name, unknown)
		}
		name := tags[0]
		if name == "" {
			name = f.Name
		}
		names = append(names, name)
		positions = append(positions, i)
	}
	if len(names) == 0 || len(positions) == 0 {
		return nil, nil, fmt.Errorf("struct '%v' has no exported fields", t)
	}
	return names, positions, nil
}

// GoRegisterStruct examines struct a and registers converters for it.
//
// The access to the underlying maps is not synchronized and therefore
// this function must be called during a single threaded initialization phase.
func (Py Py) GoRegisterStruct(a interface{}) error {
	_, ok := Py.getStructType(a)
	if ok {
		return fmt.Errorf("struct '%v' is already registered", reflect.TypeOf(a))
	}
	names, positions, err := getStructFields(a)
	if err != nil {
		return err
	}
	v := reflect.ValueOf(a)
	for _, pos := range positions {
		if v.Field(pos).Kind() == reflect.Struct {
			Py.GoRegisterStruct(v.Field(pos).Interface())
		}
	}
	t := reflect.TypeOf(a)
	t_struct, err := Py.createStructType(t.Name(), names)
	if err != nil {
		return err
	}
	c := GoConvConf{
		TypeOf:     a,
		TypeObject: t_struct,

		ToObject: func(Py pyPy, a interface{}) (Object, error) {
			v := reflect.ValueOf(a)
			fields := make([]interface{}, len(positions))
			for i, j := range positions {
				fields[i] = v.Field(j).Interface()
			}
			return Py.Object_CallFunction(t_struct.AsObject(), fields...)
		},

		FromObject: func(Py pyPy, o Object, a interface{}) error {
			v := reflect.ValueOf(a).Elem()
			switch v.Kind() {
			case reflect.Struct:
				if v.Type() != t {
					return Py.GoErrorConvFromObject(o, a)
				}
			case reflect.Interface:
				v = reflect.Indirect(reflect.New(t))
			default:
				return Py.GoErrorConvFromObject(o, a)
			}
			for i, name := range names {
				o_value, err := Py.Object_GetAttr(o, name)
				defer Py.DecRef(o_value)
				if err != nil {
					return err
				}
				f := v.Field(positions[i]).Addr()
				err = Py.GoFromObject(o_value, f.Interface())
				if err != nil {
					return fmt.Errorf("attr \"%s\": %s", name, err)
				}
			}
			if a, ok := a.(*interface{}); ok {
				*a = v.Interface()
			}
			return nil
		},
	}
	err = GoRegisterConversions(c)
	if err != nil {
		return err
	}
	Py.addStructType(a, t_struct)
	return nil
}

// GoDeregisterStruct removes the converters for struct a.
//
// The access to the underlying maps is not synchronized and therefore
// this function must be called during a single threaded deinitialization phase.
func (Py Py) GoDeregisterStruct(a interface{}) {
	if t, ok := Py.getStructType(a); ok {
		c := GoConvConf{
			TypeOf:     a,
			TypeObject: t,
		}
		GoDeregisterConversions(c)
		Py.delStructType(a)
		Py.DecRef(t.AsObject())
	}
}
