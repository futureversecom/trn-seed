#!/bin/sh

# Build binary
cargo build --release  --locked --features=runtime-benchmarks

# Create output folder
mkdir -p output

# Run overhead benchmarks
./target/release/seed benchmark overhead --chain=dev --execution=wasm --wasm-execution=compiled --warmup=10 --repeat=100 --weight-path=./output
