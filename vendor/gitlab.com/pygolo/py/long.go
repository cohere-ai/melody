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
// int pyLong_CheckExact(PyObject *o)
// {
//     return PyLong_CheckExact(o);
// }
import "C"
import (
	"fmt"
	"reflect"
	"regexp"
)

// Long_Type wraps the Python PyLong_Type type object.
//
// C API: https://docs.python.org/3/c-api/long.html#c.PyLong_Type
var Long_Type = TypeObject{&C.PyLong_Type}

// Long_CheckExact returns true if o is of type long, subtypes excluded.
//
// C API: https://docs.python.org/3/c-api/long.html#c.PyLong_CheckExact
func (Py Py) Long_CheckExact(o Object) bool {
	return C.pyLong_CheckExact(o.o) != 0
}

func (Py Py) long_FromLongLong(v int64) (Object, error) {
	return Py.wrap(C.PyLong_FromLongLong(C.longlong(v)))
}

func (Py Py) long_FromUnsignedLongLong(v uint64) (Object, error) {
	return Py.wrap(C.PyLong_FromUnsignedLongLong(C.ulonglong(v)))
}

func (Py Py) long_AsLongLong(o Object) (int64, error) {
	v := C.PyLong_AsLongLong(o.o)
	if v == -1 && Py.Err_Occurred() != (Object{}) {
		return 0, Py.GoCatchError()
	}
	return int64(v), nil
}

func (Py Py) long_AsUnsignedLongLong(o Object) (uint64, error) {
	v := C.PyLong_AsUnsignedLongLong(o.o)
	if C.longlong(v) == -1 && Py.Err_Occurred() != (Object{}) {
		return 0, Py.GoCatchError()
	}
	return uint64(v), nil
}

// longToObject converts a Go integer value to a Python long.
func longToObject(Py Py, a interface{}) (o Object, e error) {
	switch v := a.(type) {
	case int:
		o, e = Py.Long_FromInt(v)
	case int8:
		o, e = Py.Long_FromInt8(v)
	case int16:
		o, e = Py.Long_FromInt16(v)
	case int32:
		o, e = Py.Long_FromInt32(v)
	case int64:
		o, e = Py.Long_FromInt64(v)
	case uint:
		o, e = Py.Long_FromUint(v)
	case uint8:
		o, e = Py.Long_FromUint8(v)
	case uint16:
		o, e = Py.Long_FromUint16(v)
	case uint32:
		o, e = Py.Long_FromUint32(v)
	case uint64:
		o, e = Py.Long_FromUint64(v)
	case uintptr:
		o, e = Py.Long_FromUintptr(v)
	default:
		e = fmt.Errorf("not an integer: %v", a)
	}
	return
}

// longFromObject converts a Python long to a Go integer value.
func longFromObject(Py Py, o Object, a interface{}) (e error) {
	if !Py.Long_CheckExact(o) {
		return Py.GoErrorConvFromObject(o, a)
	}
	switch target := a.(type) {
	case *int:
		*target, e = Py.Long_AsInt(o)
	case *int8:
		*target, e = Py.Long_AsInt8(o)
	case *int16:
		*target, e = Py.Long_AsInt16(o)
	case *int32:
		*target, e = Py.Long_AsInt32(o)
	case *int64:
		*target, e = Py.Long_AsInt64(o)
	case *uint:
		*target, e = Py.Long_AsUint(o)
	case *uint8:
		*target, e = Py.Long_AsUint8(o)
	case *uint16:
		*target, e = Py.Long_AsUint16(o)
	case *uint32:
		*target, e = Py.Long_AsUint32(o)
	case *uint64:
		*target, e = Py.Long_AsUint64(o)
	case *uintptr:
		*target, e = Py.Long_AsUintptr(o)
	case *interface{}:
		*target, e = Py.Long_AsInt(o)
		if e != nil {
			*target, e = Py.Long_AsUint(o)
		}
	default:
		e = Py.GoErrorConvFromObject(o, a)
	}
	return
}

func long_error(a interface{}, e error) error {
	if e == nil {
		//lint:ignore ST1005 'Python' is a proper name
		e = fmt.Errorf("Python int doesn't fit, cannot convert to Go %s", reflect.TypeOf(a))
	} else if matched, _ := regexp.MatchString("OverflowError: can't convert negative .*", e.Error()); matched {
		//lint:ignore ST1005 'Python' is a proper name
		e = fmt.Errorf("Python int is negative, cannot convert to Go %s", reflect.TypeOf(a))
	} else if matched, _ := regexp.MatchString("OverflowError: .*", e.Error()); matched {
		//lint:ignore ST1005 'Python' is a proper name
		e = fmt.Errorf("Python int doesn't fit, cannot convert to Go %s", reflect.TypeOf(a))
	}
	return e
}

func init() {
	for _, kind := range []reflect.Kind{
		reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64,
		reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64,
		reflect.Uintptr,
	} {
		c := GoConvConf{
			Kind:       kind,
			ToObject:   longToObject,
			FromObject: longFromObject,
		}
		if err := GoRegisterConversions(c); err != nil {
			panic(err)
		}
	}
	c := GoConvConf{
		TypeObject: Long_Type,
		FromObject: longFromObject,
	}
	if err := GoRegisterConversions(c); err != nil {
		panic(err)
	}
}
