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
// int pyFloat_CheckExact(PyObject *o)
// {
//     return PyFloat_CheckExact(o);
// }
import "C"
import (
	"fmt"
	"reflect"
)

// Float_Type wraps the Python PyFloat_Type type object.
var Float_Type = TypeObject{&C.PyFloat_Type}

// Float_CheckExact returns true if o is of type float, subtypes excluded.
//
// C API: https://docs.python.org/3/c-api/float.html#c.PyFloat_CheckExact
func (Py Py) Float_CheckExact(o Object) bool {
	return C.pyFloat_CheckExact(o.o) != 0
}

// Float_FromDouble creates a float object from v.
//
// C API: https://docs.python.org/3/c-api/float.html#c.PyFloat_FromDouble
func (Py Py) Float_FromDouble(v float64) (Object, error) {
	return Py.wrap(C.PyFloat_FromDouble(C.double(v)))
}

// Float_AsDouble returns a float64 representation of the contents of object o.
//
// https://docs.python.org/3/c-api/float.html#c.PyFloat_AsDouble
func (Py Py) Float_AsDouble(o Object) (float64, error) {
	v := C.PyFloat_AsDouble(o.o)
	if v == -1.0 && Py.Err_Occurred() != (Object{}) {
		return 0, Py.GoCatchError()
	}
	return float64(v), nil
}

// Float_GetMin returns the minimum normalized positive float.
//
// C API: https://docs.python.org/3/c-api/float.html#c.PyFloat_GetMin
func (Py Py) Float_GetMin() float64 {
	return float64(C.PyFloat_GetMin())
}

// Float_GetMax returns the maximum representable finite float.
//
// C API: https://docs.python.org/3/c-api/float.html#c.PyFloat_GetMax
func (Py Py) Float_GetMax() float64 {
	return float64(C.PyFloat_GetMax())
}

// floatToObject converts a Go float value to a Python float.
func floatToObject(Py Py, a interface{}) (o Object, e error) {
	switch v := a.(type) {
	case float32:
		o, e = Py.Float_FromDouble(float64(v))
	case float64:
		o, e = Py.Float_FromDouble(v)
	default:
		e = fmt.Errorf("not a float: %v", a)
	}
	return
}

// floatFromObject converts a Python float to a Go float value.
func floatFromObject(Py Py, o Object, a interface{}) (e error) {
	if !Py.Float_CheckExact(o) {
		return Py.GoErrorConvFromObject(o, a)
	}
	var v float64
	switch target := a.(type) {
	case *float64:
		if v, e = Py.Float_AsDouble(o); e == nil {
			*target = v
			return
		}
	case *interface{}:
		if v, e = Py.Float_AsDouble(o); e == nil {
			*target = v
			return
		}
	default:
		e = Py.GoErrorConvFromObject(o, a)
	}
	return
}

func init() {
	c := GoConvConf{
		Kind:       reflect.Float64,
		TypeObject: Float_Type,
		ToObject:   floatToObject,
		FromObject: floatFromObject,
	}
	if err := GoRegisterConversions(c); err != nil {
		panic(err)
	}
}
