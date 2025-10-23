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

#ifndef __GO_MODULE_H__
#define __GO_MODULE_H__

#include "pygolo.h"

typedef struct {
	PyModuleDef def;
	void *data[];
} PygoloModuleDef;

PyModuleDef *pgl_new_moduledef(Py_ssize_t);
void pgl_del_moduledef(PyModuleDef *);
void *pgl_get_moduledef_data(PyModuleDef *);

#endif
