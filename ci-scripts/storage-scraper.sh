#!/bin/bash

# Build Seed
cargo build --locked --release

# Create output folder
mkdir -p output

# Create data folder
rm ./scripts/storage-scraper/data
mkdir ./scripts/storage-scraper/data

# Copy binary and chain spec
cp ./target/release/seed ./scripts/storage-scraper/data/binary
cp ./chain-spec/* ./scripts/storage-scraper/data/

# Run Scraper
cd ./scripts/storage-scraper
npm i
npm start


# Copy result to local output folder and to the exteral mapped output folder
cd ../../
cp ./scripts/storage-scraper/data/fork.json /output/
cp ./scripts/storage-scraper/data/fork.json ./output/