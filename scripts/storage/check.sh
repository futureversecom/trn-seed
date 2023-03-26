#!/bin/sh
set -e

HELP=$(./scripts/misc/misc.sh arg_exists --help $@)
if [ "$HELP" = "0" ]; then
    echo "Usage:"
    echo "  tools.sh storage check [option]"
    echo ""
    echo "Description:"
    echo "  Checks for storage/version differences between active branch and remote network"
    echo ""
    echo "Options: "
    echo "      --porcini   Compares active branch with Porcini"
    echo "      --root      Compares active branch with Root"
    echo "      --ci        Generated data will be copied to the external '/output' folder"
    echo "      --help      Display this usage message"
    exit 0
fi

URI=$(./scripts/misc/misc.sh get_uri --rpc --default porcini $@)
CI=$(./scripts/misc/misc.sh arg_exists --ci $@)

echo "------------------------------"
echo "Operation:    Storage Check   "
echo "Network URI:  $URI            "
echo "CI enabled:   $CI (0 is true) "

# Build Seed
cargo build --locked --release

# Create output folder
mkdir -p output

# Get and compile Subalfred
if [ ! -d "./trn-subalfred" ]; then
    git clone https://github.com/futureversecom/trn-subalfred.git
fi
cd trn-subalfred
cargo build --locked --release

# Do a storage check and save the output inside the local output folder
./target/release/subalfred check runtime --executable ../target/release/seed --chain dev --live "$URI" --property storage 2>&1 | tee ./../output/storage_results.txt
./target/release/subalfred check runtime --executable ../target/release/seed --chain dev --live "$URI" --property version 2>&1 | tee ./../output/version_results.txt

if [ "$CI" = "0" ]; then
    cp ./../output/storage_results.txt /output/storage_results.txt
    cp ./../output/version_results.txt /output/version_results.txt
fi
