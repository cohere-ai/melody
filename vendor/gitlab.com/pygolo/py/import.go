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

// Import_Import imports a module by invoking the current import hook function.
//
// C API: https://docs.python.org/3/c-api/import.html#c.PyImport_Import
func (Py Py) Import_Import(name string) (Object, error) {
	o_name, err := Py.Unicode_DecodeFSDefault(name)
	defer Py.DecRef(o_name)
	if err != nil {
		return Object{}, err
	}
	return Py.wrap(C.PyImport_Import(o_name.o))
}
