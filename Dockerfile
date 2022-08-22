FROM rustlang/rust:nightly AS builder
LABEL stage=build
ARG NODE_BUILD_DIR=/seed-build
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
    mkdir -p ${NODE_BUILD_DIR}/.cargo
ENV CARGO_HOME=${NODE_BUILD_DIR}/.cargo
RUN cargo build "--$PROFILE"

FROM debian:buster-slim
LABEL maintainer="support@centrality.ai"
LABEL org.opencontainers.image.source=https://github.com/futureversecom/seed
COPY --from=0 /seed-build/target/release/seed /usr/bin/

EXPOSE 30333 9933 9944
ENTRYPOINT ["/usr/bin/seed"]
