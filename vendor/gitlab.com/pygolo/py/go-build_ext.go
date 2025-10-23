//go:build !custom_python && py_ext && go1.19
// +build !custom_python,py_ext,go1.19

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

// #cgo linux LDFLAGS: -Wl,--allow-shlib-undefined
// #cgo darwin LDFLAGS: -Wl,-undefined -Wl,dynamic_lookup
//
// #cgo py3.6 pkg-config: python-3.6
// #cgo py3.7 pkg-config: python-3.7
// #cgo py3.8 pkg-config: python-3.8
// #cgo py3.9 pkg-config: python-3.9
// #cgo py3.10 pkg-config: python-3.10
// #cgo py3.11 pkg-config: python-3.11
// #cgo py3.12 pkg-config: python-3.12
// #cgo py3.13 pkg-config: python-3.13
// #cgo py3.14 pkg-config: python-3.14
// #cgo !py3.6,!py3.7,!py3.8,!py3.9,!py3.10,!py3.11,!py3.12,!py3.13,!py3.14 pkg-config: python3
import "C"
