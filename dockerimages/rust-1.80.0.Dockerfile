FROM docker.io/library/rust:1.80.0-bookworm as builder

ADD . ./workdir
WORKDIR "/workdir"

# This installs all dependencies that we need.
RUN apt update -y && \
    apt install build-essential git clang curl libssl-dev llvm libudev-dev make cmake protobuf-compiler pkg-config -y

# Install the right toolchain
RUN rustup show

VOLUME ["/output"]
