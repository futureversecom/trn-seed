FROM --platform=linux/amd64 docker.io/library/rust:1.80.0-bullseye as builder

RUN apt update -y && apt install -y \
	build-essential \
	git \
	clang \
	curl \
	libssl-dev \
	llvm \
	libudev-dev \
	make \
	cmake \
	protobuf-compiler \
	pkg-config \
	python3 \
	python3-pip \
	python3-venv

# install rust toolchain
RUN rustup show

WORKDIR /workdir
COPY . /workdir/

ARG network=porcini

# install deps & start script to switch branches & build the node with `runtime-benchmarks` flag and get state
RUN pip install -r ./scripts/requirements.txt
RUN python3 ./scripts/get_state.py --config ./scripts/networks/${network}.yaml

# copy binary
RUN cp ./target/release/seed ./output/binary

# ==============================================================================
# Multistage build
# ==============================================================================

FROM --platform=linux/amd64 docker.io/library/debian:bullseye-slim AS run

LABEL maintainer="support@centrality.ai"
LABEL org.opencontainers.image.source=https://github.com/futureversecom/seed

RUN apt update -y && apt install curl -y

COPY --from=0 /workdir/output /output
WORKDIR /output
EXPOSE 30333 9944

CMD /output/binary --chain=/output/fork.json --alice --tmp --unsafe-rpc-external --rpc-port=9944 --rpc-cors=all
