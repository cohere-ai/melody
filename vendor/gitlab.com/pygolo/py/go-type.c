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

#include "_cgo_export.h"

// Ideally we'd use PyType_FromModuleAndSpec but it's not
// available in Python < 3.9 therefore we go another way.
int setTypeHandle(PyTypeObject *tp, void *handle)
{
	if (tp->tp_methods)
		return -1;

	tp->tp_methods = PyMem_Calloc(1, sizeof(PyMethodDef));
	if (!tp->tp_methods)
		return -1;

	// Piggyback the handle into the methods' sentinel,
	// it won't disturb the interpreter
	tp->tp_methods[0].ml_meth = handle;
	return 0;
}

void *getTypeHandle(PyTypeObject *tp)
{
	if (!tp->tp_methods)
		return NULL;
	if (tp->tp_methods[0].ml_name)
		return NULL;
	return (void *) tp->tp_methods[0].ml_meth;
}
