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
// #include "go-type.h"
//
// PyTypeObject *GoFunction_Type(void);
import "C"
import (
	"fmt"
	"reflect"
)

// GoFunction_Type returns the GoFunction type object.
func (Py Py) GoFunction_Type() TypeObject {
	return Py.getState().GoFunction_Type
}

// GoFunction_Check returns true if o is of type GoFunction_Type.
func (Py Py) GoFunction_Check(o Object) bool {
	return o.Type() == Py.GoFunction_Type()
}

// GoFunction describes how a Go function is exposed to Python.
type GoFunction struct {
	// Fn holds a reference to the Go function being exposed.
	Fn interface{}

	// GoAttrs provides the get/set attributes protocol of the function object.
	GoAttrs

	// InterpreterAccess controls whether or not the exposed
	// function is allowed to access the interpreter by default.
	//
	// Setting this to true is equivalent to wrapping the function
	// between Py.GoEnterPython() and Py.GoLeavePython().
	InterpreterAccess bool
}

//export pgl_call
func pgl_call(o_, args_, kwargs_ *C.PyObject) *C.PyObject {
	Py, err := extendObject(Object{o_})
	if err != nil {
		Py.GoSetError(err)
		return nil
	}
	o := getPygoloObjectHandle(o_).Get().(*GoFunction)
	args := Object{args_}
	kwargs := Object{kwargs_}

	if kwargs_ != nil {
		length, err := Py.Object_Length(kwargs)
		if err != nil {
			Py.GoSetError(err)
			return nil
		}
		if length > 0 {
			Py.Err_SetString(Exc_TypeError, "Unexpected keyword arguments")
			return nil
		}
	}

	fn := reflect.ValueOf(o.Fn)
	fn_t := fn.Type()
	num_in := fn_t.NumIn()
	num_out := fn_t.NumOut()
	py_t := reflect.TypeOf(Py)

	ins := make([]reflect.Value, 0, num_in)
	a_ins := make([]interface{}, 0, num_in)
	a_outs := make([]interface{}, 0, num_out)

	for i := 0; i < num_in; i++ {
		in_t := fn_t.In(i)
		if in_t == py_t {
			ins = append(ins, reflect.ValueOf(Py))
		} else if i < num_in-1 || !fn_t.IsVariadic() {
			in := reflect.New(in_t)
			ins = append(ins, reflect.Indirect(in))
			a_ins = append(a_ins, in.Interface())
		} else {
			length, err := Py.Object_Length(args)
			if err != nil {
				Py.GoSetError(err)
				return nil
			}
			for len(a_ins) < length {
				in := reflect.New(in_t.Elem())
				ins = append(ins, reflect.Indirect(in))
				a_ins = append(a_ins, in.Interface())
			}
		}
	}

	err = Py.Arg_ParseTuple(args, a_ins...)
	if err != nil {
		Py.GoSetError(err)
		return nil
	}

	if !o.InterpreterAccess {
		Py.GoLeavePython()
	}
	outs, err := safeCall(fn, ins)
	if !o.InterpreterAccess {
		Py.GoEnterPython()
	}

	if err != nil {
		Py.Err_SetString(Exc_RuntimeError, err.Error())
		return nil
	}
	for _, out := range outs {
		a_outs = append(a_outs, out.Interface())
	}

	for _, a_in := range a_ins {
		if o_in, ok := a_in.(Object); ok {
			Py.DecRef(o_in)
		}
	}

	if num_out > 0 {
		var err error
		if outs[num_out-1].Type() == reflect.TypeOf(&err).Elem() {
			if err, ok := a_outs[num_out-1].(error); ok {
				Py.GoSetError(err)
				return nil
			}
			a_outs = a_outs[:num_out-1]
		}
	}

	var o_ret Object
	if len(a_outs) > 1 {
		o_ret, err = Py.Tuple_Pack(a_outs...)
	} else if len(a_outs) > 0 {
		o_ret, err = Py.GoToObject(a_outs[0])
	} else {
		o_ret = Py.NewRef(None)
	}
	if err != nil {
		Py.Err_SetString(Exc_TypeError, err.Error())
		return nil
	}
	return o_ret.o
}

func safeCall(fn reflect.Value, ins []reflect.Value) (outs []reflect.Value, err error) {
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("panic: %v", r)
		}
	}()
	return fn.Call(ins), nil
}

func (Py Py) newGoFunction_Type() (TypeObject, error) {
	t := TypeObject{C.GoFunction_Type()}
	defer Py.DecRef(t.AsObject())
	if t.t == nil {
		return TypeObject{}, Py.GoCatchError()
	}
	err := Py.setTypeState(t)
	if err != nil {
		return TypeObject{}, err
	}
	c := GoConvConf{
		TypeObject: t,
		FromObject: funcFromObject,
	}
	err = GoRegisterConversions(c)
	if err != nil {
		return TypeObject{}, err
	}
	Py.IncRef(t.AsObject())
	return t, nil
}

func (Py Py) validateGoFunction(fn GoFunction) error {
	if fn.Fn == nil {
		return Py.GoErrorConvToObject(fn.Fn, Py.GoFunction_Type())
	}
	fn_t := reflect.TypeOf(fn.Fn)
	if fn_t.Kind() != reflect.Func {
		return Py.GoErrorConvToObject(fn.Fn, Py.GoFunction_Type())
	}
	if reflect.ValueOf(fn.Fn).IsNil() {
		return Py.GoErrorConvToObject(nil, Py.GoFunction_Type())
	}
	py_t := reflect.TypeOf(Py)
	py_ptr_t := reflect.PtrTo(py_t)
	py_slice_t := reflect.SliceOf(py_t)
	py_ptr_slice_t := reflect.PtrTo(py_slice_t)
	py_slice_ptr_t := reflect.SliceOf(py_ptr_t)
	found := false
	for i := 0; i < fn_t.NumIn(); i++ {
		switch fn_t.In(i) {
		case py_t:
			if found {
				err := Py.GoErrorConvToObject(fn.Fn, Py.GoFunction_Type())
				return fmt.Errorf("%s: cannot handle multiple %v parameters", err, fn_t.In(i))
			}
			found = true
		case py_ptr_t, py_slice_t, py_ptr_slice_t, py_slice_ptr_t:
			err := Py.GoErrorConvToObject(fn.Fn, Py.GoFunction_Type())
			if i < fn_t.NumIn()-1 || !fn_t.IsVariadic() {
				return fmt.Errorf("%s: cannot handle %v parameters", err, fn_t.In(i))
			}
			return fmt.Errorf("%s: cannot handle a variable number of %v parameters", err, fn_t.In(i).Elem())
		}
	}
	return nil
}

// funcToObject wraps a Go function in a Python object.
func funcToObject(Py Py, a interface{}) (Object, error) {
	fn, ok := a.(GoFunction)
	if !ok {
		fn = GoFunction{Fn: a}
	}
	err := Py.validateGoFunction(fn)
	if err != nil {
		return Object{}, err
	}
	if fn.GoAttrs == nil {
		fn.GoAttrs = GoAttrsMap{}
	}
	err = fn.GoAttrs.PySetAttr(Py, "__doc__", reflect.TypeOf(fn.Fn).String())
	if err != nil {
		return Object{}, err
	}
	err = fn.GoAttrs.PySetAttr(Py, "__class__", Py.GoFunction_Type())
	if err != nil {
		return Object{}, err
	}
	return newPygoloObject(Py.GoFunction_Type(), &fn)
}

// funcFromObject returns the Go function that was wrapped in a Python object.
func funcFromObject(Py Py, o Object, a interface{}) error {
	if !Py.GoFunction_Check(o) {
		return Py.GoErrorConvFromObject(o, a)
	}
	fn_, ok := getPygoloObjectHandle(o.o).Get().(*GoFunction)
	if !ok {
		return Py.GoErrorConvFromObject(o, a)
	}

	fn := reflect.ValueOf(fn_.Fn)
	dest := reflect.ValueOf(a)

	switch dest.Elem().Kind() {
	case reflect.Interface:
	case reflect.Func:
		if dest.Elem().Type() != fn.Type() {
			return fmt.Errorf("cannot convert a Python <%s> to Go <%s>", fn.Type(), dest.Elem().Type())
		}
	default:
		return Py.GoErrorConvFromObject(o, a)
	}
	dest.Elem().Set(fn)
	return nil
}

func init() {
	cc := []GoConvConf{
		{
			Kind:     reflect.Func,
			TypeOf:   GoFunction{},
			ToObject: funcToObject,
		}, {
			Kind:       reflect.Func,
			FromObject: funcFromObject,
		},
	}
	if err := GoRegisterConversions(cc...); err != nil {
		panic(err)
	}
}
