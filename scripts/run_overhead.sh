#!/bin/bash
cargo build --release  --locked --features=runtime-benchmarks
mkdir -p output
./target/release/seed benchmark overhead --chain=dev --execution=wasm --wasm-execution=compiled --warmup=10 --repeat=100 --weight-path=./output
