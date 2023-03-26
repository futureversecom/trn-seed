#!/bin/sh
set -e

HELP=$(./scripts/misc/misc.sh arg_exists --help $@)
if [ "$HELP" = "0" ]; then
    echo "Usage:"
    echo "  tools.sh runtime try [option]"
    echo ""
    echo "Description:"
    echo "  Fetches remote network data and runs the try-runtime procedure"
    echo ""
    echo "Options: "
    echo "      --porcini   Fetches data from Porcini(default)"
    echo "      --root      Fetches data from Root"
    echo "      --ci        Try-runtime output will be saved to the external '/output' folder"
    echo "      --help      Display this usage message"
    exit 0
fi

URI=$(./scripts/misc/misc.sh get_uri --wss --default porcini $@)
CI=$(./scripts/misc/misc.sh arg_exists --ci $@)

echo "------------------------------"
echo "Operation:    Runtime Try     "
echo "Network URI:  $URI            "
echo "CI enabled:   $CI (0 is true) "

# Build Seed
cargo build --locked --release --features try-runtime

# Create output folder
mkdir -p output

# Run try-runtime procedure
./target/release/seed try-runtime --chain dev on-runtime-upgrade live --uri $URI 2>&1 | tee ./output/try_runtime_results.txt

if [ "$CI" = "0" ]; then
    cp ./output/try_runtime_results.txt /output/try_runtime_results.txt
fi
