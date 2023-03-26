display_usage () {
    echo "Usage: tools.sh benchmark [subcommand]"
    echo "Subcommands: "
    echo "      pallets"
    echo "      overhead"
}

SUB_COMMAND="$1"
shift;

case "$SUB_COMMAND" in
    pallets)    ./scripts/benchmark/pallets.sh "$@";;
    overhead)   ./scripts/benchmark/overhead.sh "$@";;
    *)          echo "Error: unrecognized command >>$SUB_COMMAND<<"; display_usage;;
esac
