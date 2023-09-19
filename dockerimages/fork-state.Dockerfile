FROM docker.io/library/rust:1.67.0-bullseye as builder

# This installs all dependencies that we need.
RUN apt update -y && \
	apt install build-essential git clang curl libssl-dev llvm libudev-dev make cmake protobuf-compiler pkg-config python3 python3-pip python3-venv -y

# Install the right toolchain
RUN rustup show

# Copy local files to workdir
ADD . ./workdir
WORKDIR "/workdir"
ARG network=porcini

# Install dependencies
RUN pip install -r ./scripts/requirements.txt

# Start the script to build the node with `runtime-benchmarks` flag and get state
RUN python3 ./scripts/get_state.py --config ./scripts/networks/${network}.yaml

# Copy binary
RUN cp ./target/release/seed ./output/binary

# Multistage build
FROM docker.io/library/debian:bullseye-slim AS run
RUN apt update -y && apt install curl -y
LABEL maintainer="support@centrality.ai"
LABEL org.opencontainers.image.source=https://github.com/futureversecom/seed
COPY --from=0 /workdir/output /output
WORKDIR "/output"
VOLUME ["/node-data"]
EXPOSE 30333 9933 9944
# Set the default command to run your application
CMD ["/output/binary", "--chain=/output/fork.json", "--alice", "--tmp", "--unsafe-ws-external", "--unsafe-rpc-external", "--rpc-cors=all"]