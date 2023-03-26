#!/bin/sh
set -e

display_usage () {
    echo "USAGE:"
    echo "  tools.sh storage <SUBCOMMAND>"
    echo ""
    echo "SUBCOMMANDS:"
    echo "      check       Checks for storage/version differences between active branch and remote network"
    echo "      fetch       Fetches remote network storage and builds a chain specification using it"
}

SUBCOMMAND="$1"
shift;

case "$SUBCOMMAND" in
    check)          ./scripts/storage/check.sh "$@";;
    fetch)          ./scripts/storage/fetch.sh "$@";;
    "" | "--help")  display_usage;;
    *)              echo "Error: unrecognized command >>$SUBCOMMAND<<"; display_usage;;
esac
