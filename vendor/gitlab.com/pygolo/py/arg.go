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
import "fmt"

// Arg_ParseTuple parses positional parameters into local variables.
//
// C API: https://docs.python.org/3/c-api/arg.html#c.PyArg_ParseTuple
func (Py Py) Arg_ParseTuple(o Object, args ...interface{}) error {
	length, err := Py.Object_Length(o)
	if err != nil {
		return err
	}
	if length != len(args) {
		s := "arguments"
		if len(args) == 1 {
			s = "argument"
		}
		w := "were"
		if length == 1 {
			w = "was"
		}
		msg := fmt.Sprintf("takes %d positional %s but %d %s given", len(args), s, length, w)
		return &GoError{"TypeError", msg, nil}
	}
	for i, arg := range args {
		item, err := Py.Tuple_GetItem(o, i)
		if err != nil {
			return err
		}
		err = Py.GoFromObject(item, arg)
		if err != nil {
			return fmt.Errorf("arg #%d: %s", i, err)
		}
	}
	return nil
}
