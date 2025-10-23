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

#include <assert.h>
#include "_cgo_export.h"

PyObject *
pgl_new_object(PyTypeObject *type)
{
	PygoloObject *o = PyObject_GC_New(PygoloObject, type);
	if (o) {
		assert(Py_TYPE(o)->tp_basicsize > sizeof(PygoloObject));
		memset(o->data, 0, Py_TYPE(o)->tp_basicsize - sizeof(PygoloObject));
	}
	return (PyObject *) o;
}

void
pgl_del_object(PyObject *self)
{
	PyTypeObject *tp = Py_TYPE(self);
	PyObject_GC_UnTrack(self);
	tp->tp_free(self);
}

int
pgl_traverse_object(PyObject *self, visitproc visit, void *arg)
{
	Py_VISIT(Py_TYPE(self));
	return 0;
}

void *
pgl_get_object_data(PyObject *o)
{
	return ((PygoloObject *) o)->data;
}
