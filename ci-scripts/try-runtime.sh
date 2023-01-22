#!/bin/bash

# Build Seed
cargo build --locked --release --features try-runtime

# Create output folder
mkdir -p output

# Do try-runtime
./target/release/seed try-runtime --chain dev on-runtime-upgrade live --uri wss://porcini.au.rootnet.app:443/archive/ws 2>&1 | tee ./output/try_runtime_results.txt

# Copy to exterally mapped output folder
cp ./output/try_runtime_results.txt /output/try_runtime_results.txt
