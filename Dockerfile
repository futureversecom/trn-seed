FROM  rustlang/rust:nightly AS builder
ARG NODE_BUILD_DIR=/root-network
ARG PROFILE=release
ARG RUST_NIGHTLY=nightly-2022-03-01
ARG RUST_VERSION=stable

WORKDIR ${NODE_BUILD_DIR}
COPY . $NODE_BUILD_DIR

RUN apt-get update && \
    apt-get -y install apt-utils cmake pkg-config libssl-dev git clang libclang-dev && \
    rustup uninstall nightly && \
    rustup install $RUST_VERSION && \
    rustup install $RUST_NIGHTLY && \
    rustup default $RUST_VERSION && \
    rustup target add --toolchain $RUST_NIGHTLY wasm32-unknown-unknown && \
    rustup target add --toolchain $RUST_VERSION x86_64-unknown-linux-musl && \
    mkdir -p ${NODE_BUILD_DIR}/.cargo
ENV CARGO_HOME=${NODE_BUILD_DIR}/.cargo
RUN cargo build "--$PROFILE"

FROM gcr.io/distroless/cc
LABEL maintainer="support@centrality.ai"
LABEL org.opencontainers.image.source=https://github.com/futureversecom/root-network
COPY --from=0 /root-network/target/release/root-node /usr/local/bin

EXPOSE 30333 9933 9944
ENTRYPOINT ["/usr/local/bin/root-node"]
