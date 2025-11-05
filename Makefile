TOKENIZERS_VERSION = v0.9.1
UNAME := $(shell uname)
ARCH := $(shell uname -m)
mod-install-tokenizers:
	@echo "-- installing libtokenizers.a at ~/go/pkg/mod/github.com/cohere-ai/tokenizers@${TOKENIZERS_VERSION}/libtokenizers.a..."
	@curl -fsSL https://github.com/cohere-ai/tokenizers/releases/download/${TOKENIZERS_VERSION}/libtokenizers.${UNAME}-${ARCH}.tar.gz | tar xvz
	@mv libtokenizers.a ~/go/pkg/mod/github.com/cohere-ai/tokenizers@${TOKENIZERS_VERSION}/
	@echo "-- installed libtokenizers.a"

install-tokenizers:
	@echo "-- installing libtokenizers.a at ./golang/vendor/github.com/cohere-ai/tokenizers/libtokenizers.a..."
	@curl -fsSL https://github.com/cohere-ai/tokenizers/releases/download/${TOKENIZERS_VERSION}/libtokenizers.${UNAME}-${ARCH}.tar.gz | tar xvz
	@mv libtokenizers.a golang/vendor/github.com/cohere-ai/tokenizers/
	@echo "-- installed libtokenizers.a"

check-install-tokenizers:
	@if [ ! -e "./golang/vendor/github.com/cohere-ai/tokenizers/libtokenizers.a" ]; then \
  		$(MAKE) install-tokenizers; \
	fi;

golang-lint:
	cd golang && golangci-lint run --config=.golangci.yaml ./...

golang-test: check-install-tokenizers
	cd golang && go test ./...

golang-bindings-test: check-install-tokenizers rust-build
	cd go-bindings && go test -v ./...

rust-test:
	cd rust && cargo test --verbose

rust-lint:
	cd rust && cargo clippy -- -D warnings

rust-format:
	cd rust && cargo fmt

rust-build:
	cd rust && cargo clean && cargo build --release

python-bindings:
	cd rust && maturin develop --features python_ffi
