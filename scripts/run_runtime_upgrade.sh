#!/bin/bash

# Build binary
cargo build --locked --release

mkdir ./scripts/runtime-upgrade/data/
cp ./target/release/wbuild/seed-runtime/seed_runtime.compact.compressed.wasm ./scripts/runtime-upgrade/data/test.wasm

cd ./scripts/runtime-upgrade
npm i
npm start