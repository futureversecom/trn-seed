FROM docker.io/library/rust:1.67.0-bullseye as builder

# Copy local files to workdir
ADD . ./workdir
WORKDIR "/workdir"

# This installs all dependencies that we need.
RUN apt update -y && \
	apt install build-essential git clang curl libssl-dev llvm libudev-dev make cmake protobuf-compiler pkg-config python3 python3-pip python3-venv -y

# Install the right toolchain
RUN rustup show

# Start the script
RUN pip install -r ./scripts/requirements.txt

RUN python3 ./scripts/get_state.py --config ./scripts/networks/porcini.yaml

VOLUME ["/output"]
