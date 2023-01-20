#!/bin/bash
# Build binary
cargo build --locked --release

# Get Porcini ChainSpec
mkdir -p output
./ci-scripts/storage-scraper.sh

# Run Node
./target/release/seed --chain ./output/fork.json --alice --force-authoring --tmp --rpc-cors=all --ws-external