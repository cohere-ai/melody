TOKENIZERS_VERSION = v0.9.1
UNAME := $(shell uname)
ARCH := $(shell uname -m)
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

rust-test:
	cd rust && cargo test --verbose

rust-lint:
	cd rust && cargo clippy -- -D warnings

rust-format:
	cd rust && cargo fmt

rust-build:
	cd rust && cargo build --release
