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

#define container_of(ptr, type, member) ({     \
	void *__ptr = (void *)(ptr);                 \
	void *__mptr = (void *)&((type *)0)->member; \
	(type *)(__ptr - __mptr);                    \
})

PyModuleDef *
pgl_new_moduledef(Py_ssize_t size)
{
	PyModuleDef init = { PyModuleDef_HEAD_INIT };
	PygoloModuleDef *def = PyMem_Calloc(1, sizeof(PygoloModuleDef) + size);
	if (!def) {
		return NULL;
	}
	def->def = init;
	def->def.m_size = size;
	def->def.m_slots = PyMem_Calloc(1 + 1, sizeof(PyModuleDef_Slot));
	if (!def->def.m_slots) {
		PyMem_Free(def);
		return NULL;
	}
	def->def.m_slots[0].slot = Py_mod_exec;
	def->def.m_slots[0].value = initModule;
	def->def.m_free = (freefunc) delModule;
	return &def->def;
}

void
pgl_del_moduledef(PyModuleDef *def)
{
	PyMem_Free(def->m_slots);
	PyMem_Free(container_of(def, PygoloModuleDef, def));
}

void *
pgl_get_moduledef_data(PyModuleDef *def)
{
	return container_of(def, PygoloModuleDef, def)->data;
}
