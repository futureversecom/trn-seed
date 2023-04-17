#!/bin/bash

. ./scripts/getoptions.sh
VERSION=0.1

TrapQuit() {
    # Clean
    kill $PID > /dev/null 2>&1
    rm -rf "$TEMP_FOLDER"
    
    if [ "$IGNORE_TAG" = "0" ]; then
        git checkout "$CURRENT_BRANCH" > /dev/null 2> /dev/null
    fi
}
trap TrapQuit EXIT

inputs_arguments() {
    setup   REST help:usage -- "Usage: ./scripts/run_benchmark.sh [options]... [arguments]..." ''
    msg -- 'Options:'
    param   TAG             -t  --tag       init:="latest"      -- "Specifies what tag to use to get the state from Porcini/Root. Default is latest"
    param   CHAIN           -c  --chain     init:="porcini"     -- "What chain to use. Default is Porcini"
    flag    IGNORE_TAG      -i              init:=0             -- "Ignores the TAG param and no branch switching will happen"
    flag    SKIP_SPEC           --skip-spec init:=0             -- "Skips spec creation"
    flag    SKIP_SNAP           --skip-snap init:=0             -- "Skips snap creation"
    param   OUTPUT_FOLDER   -o  --output    init:="./output"    -- "Folder where all the generated files will be stored"
    disp    :usage  -h  --help
    disp    VERSION     --version
}
eval "$(getoptions inputs_arguments - "$0") exit 1"

TEMP_FOLDER="./tmp"
OUTPUT_FILE="${TEMP_FOLDER}/temp.txt"
NODE_DATA="${TEMP_FOLDER}/node-data"
CURRENT_BRANCH="$(eval git branch --show-current)"
PYTHON_FILE="${TEMP_FOLDER}/python.py"

CHAIN_SNAP_PATH="${OUTPUT_FOLDER}/${CHAIN}_snap"
CHAIN_SPEC_PATH="${OUTPUT_FOLDER}/${CHAIN}_chain_state.json"
DEV_SPEC_PATH="${OUTPUT_FOLDER}/dev_chain_state.json"
MODULE_METADATA_PATH="${OUTPUT_FOLDER}/module_metadata.json"
BINARY_PATH="${OUTPUT_FOLDER}/binary"

FETCH_MODUE_METADATA_SCRIPT=$(cat ./scripts/fetch_module_metadata.py)
POPULATE_BASE_CHAIN_SCRIPT=$(cat ./scripts/populate_base_chain.py)

tag_checkout() {
    if [ "$IGNORE_TAG" = "1" ]; then
        echo "No tag checkout will happen"
        return 0;
    fi
    
    TAGS=($(eval git tag --sort=-creatordate))
    
    if [ "$TAG" = "latest" ]; then
        TAG="${TAGS[0]}"
    fi
    
    for ELEM in "${TAGS[@]}"; do
        if [ "$TAG" == "$ELEM" ]; then
            git checkout "$TAG" 2> /dev/null
            return 0;
        fi
    done
    
    echo "Uknown tag"
    exit 1
}

build_and_run_node() {
    echo "Building seed binary in release mode. This might take a while..."
    cargo build --release --locked --features try-runtime 2> /dev/null
    cp ./target/release/seed "$BINARY_PATH"
    
    ./target/release/seed --chain $CHAIN -d "$NODE_DATA" --sync warp &> $OUTPUT_FILE &
    PID=$!
}

wait_for_download() {
    echo "Waiting for the latest state to be avaialble. This might take a while..."
    while :
    do
        if awk '/Warp sync is complete/ {flag=1; exit} END {exit !flag}' "$OUTPUT_FILE"; then
            break
        fi
        sleep 1
    done
}

get_snapshot_and_chain_spec() {
    if [ "$SKIP_SNAP" == "0" ]; then
        echo "Creating $CHAIN snapshot"
        ./target/release/seed try-runtime on-runtime-upgrade live -s "$CHAIN_SNAP_PATH" -u "ws://127.0.0.1:9944" > /dev/null 2>&1
    else
        echo "Skipping $CHAIN snapshot creation"
    fi
    
    kill $PID
    if [ "$SKIP_SPEC" == "0" ]; then
        echo "Creating $CHAIN chain specification"
        sleep 1
        ./target/release/seed export-state --chain $CHAIN -d $NODE_DATA > "$CHAIN_SPEC_PATH" 2> /dev/null
    else
        echo "Skipping $CHAIN chain specification creation"
    fi
}

get_module_metata_data() {
    echo "Creating Module medata data file"
    echo "$FETCH_MODUE_METADATA_SCRIPT" > "$PYTHON_FILE"
    python "$PYTHON_FILE" "$MODULE_METADATA_PATH"
}

build_usable_chain_spec() {
    echo "Creating a usable chain spec"
    $BINARY_PATH build-spec --chain dev --raw --disable-default-bootnode > "$DEV_SPEC_PATH" 2> /dev/null
    
    echo "$POPULATE_BASE_CHAIN_SCRIPT" > "$PYTHON_FILE"
    python "$PYTHON_FILE" "$DEV_SPEC_PATH" "$CHAIN_SPEC_PATH" "$MODULE_METADATA_PATH"
}

rm -rf "$TEMP_FOLDER"
mkdir -p "$OUTPUT_FOLDER" "$TEMP_FOLDER"

# Install dependenices
python -m venv ./scripts/penv
source ./scripts/penv/bin/activate
echo "Installing Python dependencies"
pip install -r ./scripts/requirements.txt > /dev/null 2>&1

tag_checkout
build_and_run_node
wait_for_download

get_module_metata_data
get_snapshot_and_chain_spec
build_usable_chain_spec

echo "Done :)"
