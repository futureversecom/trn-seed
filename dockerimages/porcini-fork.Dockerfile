FROM docker.io/library/rust:1.65.0-buster as builder

ADD . ./workdir
WORKDIR "/workdir"

# This installs all dependencies that we need.
RUN apt update -y && \
    apt install build-essential git clang curl libssl-dev llvm libudev-dev make cmake protobuf-compiler pkg-config -y

# Install the right toolchain
RUN rustup show

# Install Node and NPM
RUN curl -sL https://deb.nodesource.com/setup_18.x | bash -
RUN apt install nodejs && node --version && npm --version

# RCP - WSS - P2P
EXPOSE 9933 9944 30333

ENTRYPOINT ["./scripts/run_porcini_fork.sh"]
