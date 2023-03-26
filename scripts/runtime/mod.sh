#!/bin/sh
set -e

display_usage () {
    echo "USAGE:"
    echo "  tools.sh runtime <SUBCOMMAND>"
    echo ""
    echo "SUBCOMMANDS: "
    echo "      try         Fetches remote network data and runs the try-runtime procedure"
    echo "      upgrade     Upgrades local running node with active branch code (basically a script that runs the runtime upgrade extrinsic)"
}

SUBCOMMAND="$1"
shift;

case "$SUBCOMMAND" in
    try)            ./scripts/runtime/try.sh "$@";;
    upgrade)        ./scripts/runtime/upgrade.sh "$@";;
    "" | "--help")  display_usage;;
    *)              echo "Error: unrecognized command >>$SUBCOMMAND<<"; display_usage;;
esac
