//go:build unix || darwin || freebsd || linux || openbsd
// +build unix darwin freebsd linux openbsd

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

import (
	"fmt"
	"os"
	"strings"
	"syscall"
)

// fileToObject converts a Go *os.File to a Python file.
func fileToObject(Py Py, a interface{}) (o Object, e error) {
	var file GoFile
	switch a := a.(type) {
	case GoFile:
		file = a
	case *os.File:
		file = GoFile{File: a}
	default:
		return Object{}, Py.GoErrorConvFromObject(o, a)
	}
	if file.Mode == "" {
		file.Mode = "rt"
	}
	if file.Encoding == "" && (strings.Contains(file.Mode, "t") || !strings.Contains(file.Mode, "b")) {
		file.Encoding = "utf-8"
	}
	o_builtins, err := Py.Import_Import("builtins")
	defer Py.DecRef(o_builtins)
	if err != nil {
		return Object{}, err
	}
	o_open, err := Py.Object_GetAttr(o_builtins, "open")
	defer Py.DecRef(o_open)
	if err != nil {
		return Object{}, err
	}
	fd, err := syscall.Dup(int(file.File.Fd()))
	if err != nil {
		return Object{}, fmt.Errorf("fd %v dup error: %w", file.File.Fd(), err)
	}
	o_file, err := Py.Object_Call(o_open, GoArgs{fd}, GoKwArgs{"mode": file.Mode, "encoding": file.Encoding})
	if err != nil {
		syscall.Close(fd)
		return Object{}, err
	}
	return o_file, nil
}

// fileFromObject converts a Python file to a Go *os.File.
func fileFromObject(Py Py, o Object, a interface{}) (e error) {
	o_fileno, err := Py.Object_GetAttr(o, "fileno")
	defer Py.DecRef(o_fileno)
	if err != nil {
		return Py.GoErrorConvFromObject(o, a)
	}
	o_fd, err := Py.Object_CallFunction(o_fileno)
	defer Py.DecRef(o_fd)
	if err != nil {
		return err
	}
	var fd int
	err = Py.GoFromObject(o_fd, &fd)
	if err != nil {
		return err
	}
	fd2, err := syscall.Dup(fd)
	if err != nil {
		return fmt.Errorf("fd %v dup error: %w", fd, err)
	}
	f := os.NewFile(uintptr(fd2), "")
	if f == nil {
		syscall.Close(fd2)
		return fmt.Errorf("invalid fd: %v", fd2)
	}
	*a.(**os.File) = f
	return nil
}
