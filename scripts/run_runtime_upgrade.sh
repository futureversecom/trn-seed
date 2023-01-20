#!/bin/bash

# Build binary
cargo build --locked --release

# Copy WASM file
rm ./scripts/runtime-upgrade/data/
mkdir ./scripts/runtime-upgrade/data/
cp ./target/release/wbuild/seed-runtime/seed_runtime.compact.compressed.wasm ./scripts/runtime-upgrade/data/test.wasm

# Build and run runtime-upgrade script
cd ./scripts/runtime-upgrade
npm i
npm start
