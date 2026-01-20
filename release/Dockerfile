# syntax=docker/dockerfile:1.3

FROM rust:1.90 as builder-rust
ARG TARGETPLATFORM
WORKDIR /workspace
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
RUN cargo build --release --features tkzrs

FROM golang:1.25 as builder-go
ARG TARGETPLATFORM
WORKDIR /workspace
COPY . .
COPY --from=builder-rust \
    /workspace/target/release/libcohere_melody.a \
    ./target/release/
# Run a simple test (see release/main.go)
RUN go run .
# gzip the release to be extracted later with `docker run ... cp ...`
RUN cd target/release && tar -czf ../../libcohere_melody.linux.tar.gz libcohere_melody.a
