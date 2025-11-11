TOKENIZERS_VERSION = v0.9.1
UNAME := $(shell uname)
ARCH := $(shell uname -m)
mod-install-tokenizers:
	@echo "-- installing libtokenizers.a at ~/go/pkg/mod/github.com/cohere-ai/tokenizers@${TOKENIZERS_VERSION}/libtokenizers.a..."
	@curl -fsSL https://github.com/cohere-ai/tokenizers/releases/download/${TOKENIZERS_VERSION}/libtokenizers.${UNAME}-${ARCH}.tar.gz | tar xvz
	@mv libtokenizers.a ~/go/pkg/mod/github.com/cohere-ai/tokenizers@${TOKENIZERS_VERSION}/
	@echo "-- installed libtokenizers.a"

install-tokenizers:
	@echo "-- installing libtokenizers.a at ./go-bindings/vendor/github.com/cohere-ai/tokenizers/libtokenizers.a..."
	@curl -fsSL https://github.com/cohere-ai/tokenizers/releases/download/${TOKENIZERS_VERSION}/libtokenizers.${UNAME}-${ARCH}.tar.gz | tar xvz
	@mv libtokenizers.a go-bindings/vendor/github.com/cohere-ai/tokenizers/
	@echo "-- installed libtokenizers.a"

check-install-tokenizers:
	@if [ ! -e "./go-bindings/vendor/github.com/cohere-ai/tokenizers/libtokenizers.a" ]; then \
  		$(MAKE) install-tokenizers; \
	fi;

golang-bindings-test: check-install-tokenizers rust-build
	cd go-bindings && go test -v ./...

rust-test:
	cargo test --verbose

rust-lint:
	cargo clippy --all-features  -- -Dwarnings

rust-format:
	cargo fmt

rust-build:
	cargo clean && cargo build --release

venv-setup:
	uv venv --allow-existing && uv pip install maturin pytest ty vllm

python-bindings: venv-setup
	uv run maturin develop --features python_ffi

python-bindings-test: venv-setup python-bindings
	uv run pytest tests

python-bindings-format:
	uvx black .