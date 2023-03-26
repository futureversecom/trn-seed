#!/bin/bash
set -e

# function to display the usage message
display_usage() {
    echo "Usage:"
    echo "  tools.sh <subcommand>"
    echo "Subcommands:"
    echo "  bench <pallets, overhead>     Run pallet or overhead benchmarks"
    echo "  storage <check, fetch>        Check for storage/version differences between active branch and remote network"
    echo "                                or fetch remote network storage and build a chain specification using it"
    echo "  runtime <try, upgrade>        Fetch remote network data and run the try-runtime procedure or upgrade local running node with active branch code"
    echo "  run <dev, porcini, root>      Start a local node with the dev, porcini or root chain specification"
    echo "  full-test                     Test if a feature or release branch would break Porcini or Root by running all previous subcommands"
    echo "  rpc                           Compares the current branch and a hash/tag/branch to find differences in their RPC implementation"
    echo "  clean                         Clean all podman artifacts like images and storage"
    echo "  help                          Display this usage message"
}

# store the first argument as the subcommand
SUBCOMMAND="$1"

# shift positional parameters to the left
shift

# match the subcommand and execute the corresponding script
case "$SUBCOMMAND" in
    bench)      ./scripts/benchmark/mod.sh "$@";;
    storage)    ./scripts/storage/mod.sh "$@";;
    runtime)    ./scripts/runtime/mod.sh "$@";;
    run)        ./scripts/misc/node.sh "$@";;
    full-test)  ./scripts/misc/test.sh "$@";;
    rpc)        ./scripts/misc/rpc.sh "$@";;
    clean)      ./scripts/misc/podman_clean.sh "$@";;
    help)       display_usage;;
    *)          echo "Error: unrecognized command >>$SUBCOMMAND<<"; display_usage;;
esac