# syntax=docker/dockerfile:1.3

FROM rust:1.90 as builder-rust
ARG TARGETPLATFORM
WORKDIR /workspace
COPY ./src ./src
COPY ./gen ./gen
COPY ./benches ./benches
COPY ./template_generation ./template_generation
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./release/strip_lto_bitcode.sh ./release/
# The strip step keeps the archive linkable by Go 1.27.4+, which rejects LTO
# sections in #cgo LDFLAGS archives.
RUN cargo build --release --features tkzrs && \
    ./release/strip_lto_bitcode.sh target/release/libcohere_melody.a

# Go 1.27.4 is the version that enforces the LTO check, so the smoke test below
# doubles as the regression gate for the strip step.
FROM golang:1.27.4 as builder-go
ARG TARGETPLATFORM
WORKDIR /workspace
COPY ./go.mod   .
COPY ./release/main.go  .
COPY ./gobindings ./gobindings
COPY ./vendor ./vendor
COPY --from=builder-rust \
    /workspace/target/release/libcohere_melody.a \
    ./target/release/
# Run a simple test (see release/main.go)
RUN go run .
# gzip the release to be extracted later with `docker run ... cp ...`
RUN cd target/release && tar -czf ../../libcohere_melody.linux.tar.gz libcohere_melody.a
