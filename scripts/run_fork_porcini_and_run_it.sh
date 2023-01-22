#!/bin/bash

# Run Scraper
./ci-scripts/storage-scraper.sh

# Run Node
./target/release/seed --chain ./output/fork.json --alice --force-authoring --tmp --rpc-cors=all --ws-external
