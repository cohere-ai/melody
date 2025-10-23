include Python.mk

GIT_ROOT := $(shell git rev-parse --show-toplevel)
PYGOLO_GOCACHE ?= $(shell $(GO) env GOCACHE)-pygolo

GO_TAGS = $(if $(PYGOLO_TAGS),-tags "$(PYGOLO_TAGS)")
GO_VERBOSE := $(if $(filter-out 0,$(V)),-v)
GO_FLAGS = $(strip $(GO_TAGS) $(GO_VERBOSE) $(PYGOLO_FLAGS))

PYTEST_FLAGS_ := $(strip $(if $(filter-out 0,$(V)),-v -s,-q) $(PYTEST_FLAGS))

GDB ?= gdb -return-child-result --nh

EXTENSIONS := test/ext.$(PY_MOD_EXT)
EXAMPLES := $(basename $(wildcard examples/*.go))

all: test

license-check:
	@bash scripts/check-license.sh

spell-check:
ifeq ($(CHECK_ONLY),1)
	test -z "`hunspell -a -d en_GB,en_US -p $(GIT_ROOT)/.hunspell_dict *.md docs/*.md | grep '^&' | tee /dev/stderr`"
else
	hunspell -d en_GB,en_US -p $(GIT_ROOT)/.hunspell_dict *.md docs/*.md
	sort $(GIT_ROOT)/.hunspell_dict > $(GIT_ROOT)/.hunspell_dict.new
	mv $(GIT_ROOT)/.hunspell_dict.new $(GIT_ROOT)/.hunspell_dict
endif

lint-prereq:
	$(GO) install golang.org/x/lint/golint@latest
	$(GO) install honnef.co/go/tools/cmd/staticcheck@latest
	$(PYTHON) -m pip install black isort

lint: PATH := $(PATH):$(shell $(GO) env GOPATH)/bin
lint:
	$(set-pygolo-gocache)
	test -z "$$(gofmt -d . | tee /dev/stderr)"
	test -z "$$(golint . | grep -vf .golint-ignore | tee /dev/stderr)"
	$(GO) vet $(GO_FLAGS) -cgocall=false .
	staticcheck $(GO_TAGS) .
	$(PYTHON) -m black -q --check . || ($(PYTHON) -m black .; false)
	$(PYTHON) -m isort -q --check . || ($(PYTHON) -m isort .; false)

pkg-config:
	# Windows doesn't come with .pc files, let's guess our way
	$(PYTHON) scripts/create-pkg-config-files.py > python3.pc
	$(PYTHON) scripts/create-pkg-config-files.py > python3-embed.pc
	$(PYTHON) scripts/create-pkg-config-files.py > python-$(PYTHON_VERSION_SHORT).pc
	$(PYTHON) scripts/create-pkg-config-files.py > python-$(PYTHON_VERSION_SHORT)-embed.pc

test-matrix-check: GO_VER := $(shell $(GO) version | cut -d" " -f3)
test-matrix-check: PY_VER := $(shell $(PYTHON) -V | cut -d" " -f2)
test-matrix-check:
	$(PYTHON) scripts/check-test-matrix.py "$(TEST_MATRIX_NAME)" "$(GO_VER)" "$(PY_VER)"

test-embed:
	$(embed-python)
	$(GO) test $(GO_FLAGS) ./test

test-embed-debug: export CGO_CFLAGS += -ggdb
test-embed-debug:
	$(embed-python)
	$(GO) test $(GO_FLAGS) -c ./test
	$(GDB) -q -x scripts/gdb-test-embed --args test.test $(if $(filter-out 0,$(V)),-test.v)

test-extend: $(EXTENSIONS)
	$(PYTHON) -m pytest $(PYTEST_FLAGS_) --benchmark-skip test

test: test-embed test-extend

benchmark-embed:
	$(embed-python)
	$(GO) test $(GO_FLAGS) -run='^$$' -bench=. ./test

benchmark-extend: $(EXTENSIONS)
	$(PYTHON) -m pytest $(PYTEST_FLAGS_) --benchmark-only test

benchmark: benchmark-embed benchmark-extend

examples: $(EXAMPLES)
ifneq ($(filter-out 0,$(RUN)),)
	@for E in $(notdir $(sort $^)); do \
		echo "### $$E ###"; \
		(cd examples && ./$$E) || exit $$?; \
	done
endif

%: %.go FORCE
	$(embed-python)
	$(GO) build $(GO_FLAGS) -o $@ ./$<

%.$(PY_MOD_EXT): % FORCE
	$(extend-python)
	$(GO) build $(GO_FLAGS) -o $@ ./$<

clean:
	rm -rf test.test
	rm -rf $(EXAMPLES)
	rm -rf $(EXTENSIONS) $(EXTENSIONS:.$(PY_MOD_EXT)=.h)

mrproper: clean pygolo-mrproper

FORCE:

.PHONY: FORCE test
