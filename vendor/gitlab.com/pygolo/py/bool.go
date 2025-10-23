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
// int pyBool_Check(PyObject *o)
// {
//     return PyBool_Check(o);
// }
//
// PyObject* py_True()
// {
//     return Py_True;
// }
//
// PyObject* py_False()
// {
//     return Py_False;
// }
import "C"
import (
	"reflect"
)

// Bool_Type wraps the Python PyBool_Type type object.
var Bool_Type = TypeObject{&C.PyBool_Type}

// True wraps the Python Py_True object.
//
// C API: https://docs.python.org/3/c-api/bool.html#c.Py_True
var True = Object{C.py_True()}

// False wraps the Python Py_False object.
//
// C API: https://docs.python.org/3/c-api/bool.html#c.Py_False
var False = Object{C.py_False()}

// Bool_Check returns true if o is of type Bool_Type.
//
// C API: https://docs.python.org/3/c-api/bool.html#c.PyBool_Check
func (Py Py) Bool_Check(o Object) bool {
	return C.pyBool_Check(o.o) != 0
}

// boolToObject converts a Go bool value to a Python bool.
func boolToObject(Py Py, a interface{}) (o Object, e error) {
	if a.(bool) {
		o = True
	} else {
		o = False
	}
	Py.IncRef(o)
	return
}

// boolFromObject converts a Python bool to a Go bool value.
func boolFromObject(Py Py, o Object, a interface{}) error {
	var value bool
	var err error

	switch target := a.(type) {
	case *bool:
		if value, err = Py.Object_IsTrue(o); err == nil {
			*target = value
		}
	case *interface{}:
		if value, err = Py.Object_IsTrue(o); err == nil {
			*target = value
		}
	default:
		err = Py.GoErrorConvFromObject(o, a)
	}
	return err
}

func init() {
	c := GoConvConf{
		Kind:       reflect.Bool,
		TypeObject: Bool_Type,
		ToObject:   boolToObject,
		FromObject: boolFromObject,
	}
	if err := GoRegisterConversions(c); err != nil {
		panic(err)
	}
}
