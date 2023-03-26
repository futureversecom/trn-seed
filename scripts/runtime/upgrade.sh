#!/bin/sh
set -e

HELP=$(./scripts/misc/misc.sh arg_exists --help $@)
if [ "$HELP" = "0" ]; then
    echo "Usage:"
    echo "  tools.sh runtime upgrade [option]"
    echo "Description:"
    echo "  Builds the WASM file from active branch and runs a runtime upgrade on the local/porcin/root network"
    echo ""
    echo "Options: "
    echo "      --local     Upgrades the local network (default)"
    echo "      --porcini   Upgrades Porcini"
    echo "      --root      Upgrades Root"
    echo "      --ci        Stores the runtime WASM file in the external '/output' folder"
    echo "      --help      Display this usage message"
    exit 0
fi

URI=$(./scripts/misc/misc.sh get_uri "--wss --default local $@")
SUDO_KEY=$(./scripts/misc/misc.sh arg_value --sudo-key $@)
CI=$(./scripts/misc/misc.sh arg_exists --ci $@)

if ! [ -z "$URI" ]; then
    export URI
fi
if ! [ -z "$SUDO_KEY" ]; then
    export SUDO_KEY
fi

echo "------------------------------"
echo "Operation:    Runtime Upgrade "
echo "URI:          $URI            "
echo "SUDO_KEY:     ${SUDO_KEY: -10}"

# Build binary
cargo build --locked --release

# Create output folder
mkdir -p output

# Copy WASM file
rm -rf ./scripts/nodejs/data/
mkdir ./scripts/nodejs/data/
cp ./target/release/wbuild/seed-runtime/seed_runtime.compact.compressed.wasm ./scripts/nodejs/data/runtime.wasm
cp ./target/release/wbuild/seed-runtime/seed_runtime.compact.compressed.wasm ./output/runtime.wasm

# Build and run runtime-upgrade script
cd ./scripts/nodejs
yarn
yarn run runtime-upgrade

if [ "$CI" = "0" ]; then
    cp ./output/runtime.wasm /output/runtime.wasm
fi