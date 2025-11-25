TOKENIZERS_VERSION = v0.9.1
UNAME := $(shell uname)
ARCH := $(shell uname -m)

#--------------------
# GOLANG THINGS
#--------------------

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

# we kind of assume that you're running this on a macOS machine - it just builds locally
release-darwin-%:
	cargo build --release --target $*-apple-darwin
	mkdir -p artifacts/darwin-$*
	cp target/$*-apple-darwin/release/libcohere_melody.* artifacts/darwin-$*
	cd artifacts/darwin-$* && \
		tar -czf libcohere_melody.darwin-$*.tar.gz libcohere_melody.*
	mkdir -p artifacts/all
	cp artifacts/darwin-$*/libcohere_melody.darwin-$*.tar.gz artifacts/all/libcohere_melody.darwin-$*.tar.gz

release-linux-%:
	docker buildx build --no-cache --platform linux/$* -f release/Dockerfile . -t melody.linux-$*
	mkdir -p artifacts/linux-$*
	docker run -v $(PWD)/artifacts/linux-$*:/mnt --entrypoint cp melody.linux-$* /workspace/libcohere_melody.linux.tar.gz /mnt/libcohere_melody.linux.tar.gz
	mkdir -p artifacts/all
	cp artifacts/linux-$*/libcohere_melody.linux.tar.gz artifacts/all/libcohere_melody.linux-$*.tar.gz

release: release-darwin-aarch64 release-darwin-x86_64 release-linux-arm64 release-linux-x86_64
	cp artifacts/all/libcohere_melody.darwin-aarch64.tar.gz artifacts/all/libcohere_melody.darwin-arm64.tar.gz
	cp artifacts/all/libcohere_melody.darwin-x86_64.tar.gz artifacts/all/libcohere_melody.darwin-x86_64.tar.gz
	cp artifacts/all/libcohere_melody.linux-arm64.tar.gz artifacts/all/libcohere_melody.linux-aarch64.tar.gz
	cp artifacts/all/libcohere_melody.linux-x86_64.tar.gz artifacts/all/libcohere_melody.linux-amd64.tar.gz

#--------------------
# RUST THINGS
#--------------------

rust-test:
	cargo test --verbose

rust-lint:
	cargo clippy --all-features  -- -Dwarnings

rust-format:
	cargo fmt

rust-build:
	cargo clean && cargo build --release


#--------------------
# PYTHON THINGS
#--------------------

venv-setup:
	uv venv --allow-existing && uv pip install maturin pytest ty vllm

python-bindings: venv-setup
	uv run maturin develop --features python_ffi

python-bindings-test: venv-setup python-bindings
	uv run pytest tests

python-bindings-format:
	uvx black .