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
#include "go-object.h"

static PyGetSetDef pgl_getset[] = {
    {"__class__", pgl_get_attr, NULL, NULL, "__class__"},
    {"__doc__", pgl_get_attr, pgl_set_attr, NULL, "__doc__"},
    {"__module__", pgl_get_attr, pgl_set_attr, NULL, "__module__"},
    {"__name__", pgl_get_attr, pgl_set_attr, NULL, "__name__"},
    {NULL} /* Sentinel */
};

static PyObject *
pgl_descr_get(PyObject *func, PyObject *obj, PyObject *type)
{
	if (obj == NULL || obj == Py_None) {
		Py_INCREF(func);
		return func;
	}
	return PyMethod_New(func, obj);
}

PyTypeObject *
GoFunction_Type(void)
{
	PyType_Slot slots[] = {
		{Py_tp_dealloc, delPygoloObject},
		{Py_tp_traverse, pgl_traverse_object},
		{Py_tp_call, pgl_call},
		{Py_tp_descr_get, pgl_descr_get},
		{Py_tp_getset, pgl_getset},
		{0, NULL} /* Sentinel */
	};

	PyType_Spec spec = {
		.name = "GoFunction",
		.flags = Py_TPFLAGS_DEFAULT | Py_TPFLAGS_HAVE_GC,
		.basicsize = sizeof(PygoloObject) + getPygoloObjectDataSize(),
		.slots = slots,
	};

	return (PyTypeObject *) PyType_FromSpec(&spec);
}
