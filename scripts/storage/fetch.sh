#!/bin/sh
set -e

HELP=$(./scripts/misc/misc.sh arg_exists --help $@)
if [ "$HELP" = "0" ]; then
    echo "Usage:"
    echo "  tools.sh storage fetch [option]"
    echo ""
    echo "Description:"
    echo "  Fetches storage data from remote network and builds a chain specification file using it."
    echo "  Flag '--run' allows to immediately run a node with the generateed chain speficaition."
    echo "  Flag '--local' fetches data from a running local node"
    echo ""
    echo "Options:"
    echo "      --local         Fetches data from local node"
    echo "      --porcini       Fetches data from Porcini"
    echo "      --root          Fetches data from Root"
    echo "      --run           Immediately runs a node with the generateed chain specification"
    echo "      --file-prefix   Adds a prefix to existing file names. Should only be called from other scripts"
    echo "      --ci            All fetched and generated data will be copied to the external '/output' folder"
    echo "      --help          Display this usage message"
    exit 0
fi

NETWORK=$(./scripts/misc/misc.sh get_network --default porcini $@)
RUN_NODE=$(./scripts/misc/misc.sh arg_exists --run $@)
CI=$(./scripts/misc/misc.sh arg_exists --ci $@)
FILE_PREFIX=$(./scripts/misc/misc.sh arg_value --file-prefix $@)
FORK_FILE_NAME="${FILE_PREFIX}fork.json"
STROAGE_FILE_NAME="${FILE_PREFIX}storage.json"
export NETWORK;

echo "----------------------------------------------"
echo "Operation:        Storage Fetch               "
echo "Target Network:   $NETWORK                    "
echo "Run new network:  $RUN_NODE (0 is true)       "
echo "CI enabled:       $CI (0 is true)             "
echo "File Prefix:      $FILE_PREFIX                "

# Build Seed
cargo build --locked --release

# Create output folder
mkdir -p output

# Create data folder
rm -rf ./scripts/nodejs/data
mkdir ./scripts/nodejs/data

# Copy binary and chain spec
cp ./target/release/seed ./scripts/nodejs/data/binary
cp ./chain-spec/* ./scripts/nodejs/data/

# Run Scraper
cd ./scripts/nodejs
yarn
yarn run fetch-chain-spec

# Copy result to local output folder
cd ../../
cp ./scripts/nodejs/data/fork.json ./output/$FORK_FILE_NAME
cp ./scripts/nodejs/data/storage.json ./output/$STROAGE_FILE_NAME

if [ "$CI" = "0" ]; then
    cp ./scripts/nodejs/data/fork.json /output/$FORK_FILE_NAME
    cp ./scripts/nodejs/data/storage.json /output/$STROAGE_FILE_NAME
fi

if [ "$RUN_NODE" = "0" ]; then
    ./target/release/seed --chain ./output/fork.json --alice --force-authoring --tmp --rpc-cors=all --rpc-max-response-size 1000
fi
