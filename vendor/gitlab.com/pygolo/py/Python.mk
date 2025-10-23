PYTHON ?= python3
GO ?= go

GOOS := $(shell $(GO) env GOOS)

PYTHON_CONFIG_VAR = $(shell $(PYTHON) -c "import sysconfig; print(sysconfig.get_config_var('$(1)') or '')")
PYTHON_VERSION_SHORT := $(call PYTHON_CONFIG_VAR,py_version_short)
PYTHON_LIBDIR := $(call PYTHON_CONFIG_VAR,LIBDIR)
PYTHON_LIBPC := $(call PYTHON_CONFIG_VAR,LIBPC)
PYTHON_FINGERPRINT := $(shell $(PYTHON) -c "import hashlib, sysconfig; print(hashlib.sha1(sysconfig.get_config_var('srcdir').encode()).hexdigest())")

ifneq ($(PYGOLO_NOTAGS),true)
	PYGOLO_TAGS += py$(PYTHON_VERSION_SHORT)
endif

ifeq ($(GOOS),windows)
PY_MOD_EXT := pyd
else
PY_MOD_EXT := so
endif

ifeq ($(GOOS),windows)
PKG_CONFIG_PATH := $(PYTHON_LIBPC);$(PKG_CONFIG_PATH)
else
PKG_CONFIG_PATH := $(PYTHON_LIBPC):$(PKG_CONFIG_PATH)
endif
export PKG_CONFIG_PATH

CGO_ENABLED := 1
export CGO_ENABLED

ifeq ($(GOOS),darwin)
	DYLD_LIBRARY_PATH := $(PYTHON_LIBDIR):$(DYLD_LIBRARY_PATH)
	export DYLD_LIBRARY_PATH

	PYTHON_PYTHONFRAMEWORKPREFIX := $(call PYTHON_CONFIG_VAR,PYTHONFRAMEWORKPREFIX)
	ifneq ($(PYTHON_PYTHONFRAMEWORKPREFIX),)
		CGO_LDFLAGS := -Wl,-rpath,$(PYTHON_PYTHONFRAMEWORKPREFIX)
		export CGO_LDFLAGS
	endif
else
	LD_LIBRARY_PATH := $(PYTHON_LIBDIR):$(LD_LIBRARY_PATH)
	export LD_LIBRARY_PATH
endif

define set-pygolo-gocache
	$(if $(PYGOLO_GOCACHE),
		$(eval $@: export GOCACHE=$(PYGOLO_GOCACHE)/$(PYTHON_FINGERPRINT)))
endef

define embed-python
	$(if $(PYTHON_LIBPC),,
		$(warning Embedding Python is not supported on this platform))
	$(set-pygolo-gocache)
	$(eval $@: PYGOLO_FLAGS += $(PYGOLO_FLAGS_EMBED))
endef

define extend-python
	$(if $(PYTHON_LIBPC),,
		$(warning Extending Python is not supported on this platform))
	$(set-pygolo-gocache)
	$(eval $@: PYGOLO_FLAGS += $(PYGOLO_FLAGS_EXTEND) -buildmode=c-shared)
	$(eval $@: PYGOLO_TAGS += py_ext)
endef

pygolo-diags: PC_FILE_FILTER := sed -n \
	-e "s/^.* from file '\(.*\)'$$/\1/gp" \
	-e "s/^.*pkgconf_.*_path.* found: \(.*\)$$/\1/gp" \
	-e "s/^pathresolve.*looking in \(.*\)$$/\1/gp"
pygolo-diags:
	$(set-pygolo-gocache)
	@$(GO) list -f {{.Version}} -m gitlab.com/pygolo/py 2>/dev/null || true
	@$(GO) version
	@echo "GO: $(GO) (`command -v -- $(GO)`)"
	@echo "GOOS: `$(GO) env GOOS`"
	@echo "GOARCH: `$(GO) env GOARCH`"
	@echo "GOCACHE: `$(GO) env GOCACHE`"
	@$(PYTHON) -V
	@echo "PYTHON: $(PYTHON) (`command -v -- $(PYTHON)`)"
	@echo "PYTHON_LIBDIR: $(PYTHON_LIBDIR)"
	@echo "Py_GIL_DISABLED: $(call PYTHON_CONFIG_VAR,Py_GIL_DISABLED)"
ifeq ($(GOOS),darwin)
	@echo "DYLD_LIBRARY_PATH: $(DYLD_LIBRARY_PATH)"
	@echo "PYTHON_PYTHONFRAMEWORKPREFIX: $(PYTHON_PYTHONFRAMEWORKPREFIX)"
else
	@echo "LD_LIBRARY_PATH: $(LD_LIBRARY_PATH)"
endif
	@echo "PYTHON_LIBPC: $(PYTHON_LIBPC)"
	@echo "PKG_CONFIG_PATH: $(PKG_CONFIG_PATH)"
ifneq ($(PYTHON_LIBPC),)
	@cd $(PYTHON_LIBPC) && ls -l python*.pc
endif
ifeq ($(realpath $(PWD)),$(realpath $(CURDIR)))
	pyenv versions 2>/dev/null || true
else
	(cd $(PWD) && pyenv versions 2>/dev/null) || true
endif
	@echo "python-$(PYTHON_VERSION_SHORT)-embed.pc ->" \
		`pkg-config --debug python-$(PYTHON_VERSION_SHORT)-embed 2>&1 | $(PC_FILE_FILTER) | tail -1`
	@echo "python-$(PYTHON_VERSION_SHORT).pc ->" \
		`pkg-config --debug python-$(PYTHON_VERSION_SHORT) 2>&1 | $(PC_FILE_FILTER) | tail -1`
	@echo "python3-embed.pc ->" \
		`pkg-config --debug python3-embed 2>&1 | $(PC_FILE_FILTER) | tail -1`
	@echo "python3.pc ->" \
		`pkg-config --debug python3 2>&1 | $(PC_FILE_FILTER) | tail -1`

pygolo-mrproper:
	$(set-pygolo-gocache)
	$(GO) clean -cache -testcache

# prevent any pygolo-* target from becoming the default target
.DEFAULT_GOAL := $(filter-out pygolo-%,$(.DEFAULT_GOAL))
