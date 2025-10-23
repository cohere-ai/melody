TOKENIZERS_VERSION = v0.9.1
UNAME := $(shell uname)
ARCH := $(shell uname -m)
install-tokenizers:
	@echo "-- installing libtokenizers.a at vendor/github.com/cohere-ai/tokenizers/libtokenizers.a..."
	@curl -fsSL https://github.com/cohere-ai/tokenizers/releases/download/${TOKENIZERS_VERSION}/libtokenizers.${UNAME}-${ARCH}.tar.gz | tar xvz
	@mv libtokenizers.a vendor/github.com/cohere-ai/tokenizers/
	@echo "-- installed libtokenizers.a"

check-install-tokenizers:
	@if [ ! -e "vendor/github.com/cohere-ai/tokenizers/libtokenizers.a" ]; then \
  		$(MAKE) install-tokenizers; \
	fi;

lint:
	golangci-lint run --config=.golangci.yaml .

test: check-install-tokenizers
	go test ./...

build-pygolo:
	go build -tags py_ext -buildmode=c-shared -o melody.so main/main.go