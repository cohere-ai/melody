# syntax=docker/dockerfile:1.3

FROM rust:1.90 as builder-rust
ARG TARGETPLATFORM
WORKDIR /workspace
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
RUN cargo build --release

FROM golang:1.25 as builder-go
ARG TARGETPLATFORM
WORKDIR /workspace
COPY ./release/go.mod   .
COPY ./release/main.go  .
COPY ./go-bindings/*.go ./go-bindings/
COPY ./go-bindings/melody.h ./go-bindings/
COPY --from=builder-rust \
    /workspace/target/release/libcohere_melody.* \
    ./target/release/
# Run a simple test (see release/main.go)
RUN LD_LIBRARY_PATH=/workspace/target/release:$LD_LIBRARY_PATH go run .
# gzip the release to be extracted later with `docker run ... cp ...`
RUN tar -czf libcohere_melody.linux.tar.gz target/release/libcohere_melody.*
