#!/bin/sh
set -e

display_usage () {
    echo "Usage:"
    echo "  tools.sh run <subcommand>"
    echo ""
    echo "Description:"
    echo "  Runs node with either dev, porcini or root chain specification"
    echo ""
    echo "Subcommands: "
    echo "      dev         Runs node with the dev chain specificaiton (default)"
    echo "      porcini     Runs node with the porcini chain specificaiton"
    echo "      root        Runs node with the root chain specificaiton"
}

HELP=$(./scripts/misc/misc.sh arg_exists --help $@)
if [ "$HELP" = "0" ]; then
    display_usage
    exit 0
fi

SUBCOMMAND=$1
shift;

# Message
echo "------------------------------"
echo "Operation: Run $SUBCOMMAND    "

case "$SUBCOMMAND" in
    dev)            cargo run -- --dev;;
    porcini)        cargo run -- --tmp --validator --chain ./chain-spec/porcini.json;;
    root)           cargo run -- --tmp --validator --chain ./chain-spec/root.json;;
    "" | "--help")  display_usage;;
    *)              echo "Error: unrecognized command >>$SUBCOMMAND<<"; display_usage;;
esac
