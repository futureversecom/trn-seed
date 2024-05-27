# Stage 1 - Build node
FROM docker.io/library/rust:1.71.0-bookworm AS builder

# Copy local files to workdir
ADD . ./workdir
WORKDIR "/workdir"

# This installs all dependencies that we need.
RUN apt update -y && \
    apt install build-essential git clang curl libssl-dev llvm libudev-dev make cmake protobuf-compiler pkg-config -y

# Install the right toolchain and build the node
RUN rustup show && cargo build --release --locked

# Stage 2 - Run node
FROM docker.io/library/debian:bookworm-slim AS run
RUN apt update -y && apt install curl -y
LABEL maintainer="The Root Network Team"
LABEL org.opencontainers.image.source=https://github.com/futureversecom/trn-seed
COPY --from=0 /workdir/target/release/seed /usr/bin/

EXPOSE 30333 9944
VOLUME ["/node-data"]
ENTRYPOINT ["/usr/bin/seed"]
