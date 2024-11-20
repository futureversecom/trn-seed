#!/bin/bash

. ./scripts/getoptions.sh

VERSION=0.1
inputs_arguments() {
    setup   REST help:usage -- "Usage: ./scripts/run_benchmark.sh [options]... [arguments]..." ''
    msg -- 'Options:'
    param   TEMPLATE_PATH           --template      init:="./scripts/pallet_template.hbs"   -- "Specifies template location"
    param   OUTPUT_FOLDER       -o  --output        init:="./runtime/src/weights"           -- "Folder where all the weight files will be stored"
    param   PALLETS             -p  --pallets       init:="*"                               -- "List of pallets that need to be bechmarked. Default is all. Example: -p \"pallet_nft pallet_echo\""
    flag    SKIP_BUILD          -S  --skip-build                                            -- "Skips the build process if set"
    param   STEPS               -s  --steps         init:=50                                -- "How many steps to do. Default is 50"
    param   REPEAT              -r  --repeat        init:=20                                -- "How many repeats to do. Default is 20"
    flag    USE_TEMPLATE        -t                                                          -- "If set then the template will be used to generate the weight files"
    flag    JUST_CUSTOM_PALLETS -c                                                          -- "Benchmarks just our own custom pallets"
    param   BINARY_LOCATION     -b                  init:="./target/release/seed"           -- "Path where the binary is located"
    flag    LIST_PALLET         -l                                                          -- "List all pallets that can be benchmarked"
    disp    :usage  -h --help
    disp    VERSION    --version
    flag    SKIP_EXCLUDED_CHECK  -e  --skip-exculded-check
}

run_benchmark() {
    echo "Pallets: ${PALLETS[@]}"
    echo "Custom Pallets: ${CUSTOM_PALLETS[@]}"
    echo "Steps: $STEPS, Repeat: $REPEAT"
    
    if [ "$LIST_PALLET" = 1 ]; then
        exit 0
    fi
    
    rm -f $ERR_FILE
    mkdir -p "$OUTPUT_FOLDER"
    
    for PALLET in "${PALLETS[@]}"; do
        if [ ! "$SKIP_EXCLUDED_CHECK" = "1" ] && is_pallet_excluded; then
            echo -e "[ ] Skipping pallet $PALLET\n";
            continue
        fi
        
        FILE_NAME="$PALLET.rs"
        TEMPLATE_NAME="${PALLET}_weights.rs"
        
        if [ "$USE_TEMPLATE" = "1" ]; then
            FILE_NAME="$TEMPLATE_NAME"
            TEMPLATE_ARG="--template $TEMPLATE_PATH";
        fi
        
        benchmark_runtime "$TEMPLATE_ARG" "$FILE_NAME"
        
        if is_custom_pallet && [ ! "$USE_TEMPLATE" = "1" ]; then
            benchmark_pallet "--template $TEMPLATE_PATH" "$TEMPLATE_NAME"
        fi

        echo ""
        
    done
}

benchmark_runtime() {
    echo "[+][runtime] Benchmarking $PALLET";

    WEIGHT_FILENAME=$(echo $2 | tr '-' '_');
    OUTPUT=$($BINARY_LOCATION benchmark pallet --chain=dev --steps=$STEPS --repeat=$REPEAT --pallet="$PALLET" --extrinsic="*" --wasm-execution=compiled --heap-pages=4096 --output "$OUTPUT_FOLDER/$WEIGHT_FILENAME" $1 2>&1 )
    if [ $? -ne 0 ]; then
        echo "$OUTPUT" >> "$ERR_FILE"
        echo "[-][runtime] Failed to benchmark $PALLET. Error written to $ERR_FILE; continuing..."
    fi
}

benchmark_pallet() {
    echo "[+][pallet] Benchmarking $PALLET";

    # remove the 'pallet-' prefix
    PALLET_FOLDER="./pallet/$(echo ${PALLET#pallet-})/src"

    OUTPUT=$($BINARY_LOCATION benchmark pallet --chain=dev --steps=$STEPS --repeat=$REPEAT --pallet="$PALLET" --extrinsic="*" --wasm-execution=compiled --heap-pages=4096 --output "$PALLET_FOLDER/weights.rs" $1 2>&1 )
    if [ $? -ne 0 ]; then
        echo "$OUTPUT" >> "$ERR_FILE"
        echo "[-][pallet] Failed to benchmark $PALLET. Error written to $ERR_FILE; continuing..."
    fi
}

is_pallet_excluded() {
    for EXCLUDED_PALLET in "${EXCLUDED_PALLETS[@]}"; do
        if [ "$EXCLUDED_PALLET" == "$PALLET" ]; then
            return 0
        fi
    done
    
    return 1
}

is_custom_pallet() {
    for CUSTOM_PALLETS in "${CUSTOM_PALLETS[@]}"; do
        if [ "$CUSTOM_PALLETS" == "$PALLET" ]; then
            return 0
        fi
    done
    return 1
}

populate_pallet_list() {
    # Manually exclude some pallets.
    EXCLUDED_PALLETS=(
        # Helper pallets
        "pallet-election-provider-support-benchmarking"
        # Pallets without automatic benchmarking
        "pallet-babe"
        "pallet-grandpa"
        "pallet-mmr"
        "pallet-offences"
        "frame-benchmarking"

        # pallet bench taking too long - use SKIP_EXCLUDED_CHECK flag to run these
        "pallet-assets"
        "pallet-election-provider-multi-phase"
        "pallet-im-online"
    )
    
    CUSTOM_PALLETS=()
    for f in ./pallet/*/Cargo.toml; do
        pallet_name=$(awk -F' = ' '$1 == "name" {print $2}' $f | tr -d '"')
        CUSTOM_PALLETS+=($pallet_name)
    done;
    
    if ! [ "$PALLETS" = "*" ]; then
        PALLETS=($PALLETS)
    fi
    if [ "$LIST_PALLET" = "1" ] || [ "$PALLETS" = "*" ]; then
        PALLETS=($($BINARY_LOCATION benchmark pallet --list --chain=dev | tail -n+2 | cut -d',' -f1 | sort | uniq| tr _ - ))
    fi
    if [ "$JUST_CUSTOM_PALLETS" = "1" ]; then
        PALLETS=("${CUSTOM_PALLETS[@]}")
    fi
}

eval "$(getoptions inputs_arguments - "$0") exit 1"

mkdir -p "$OUTPUT_FOLDER"
ERR_FILE="./benchmarking_errors.txt"

if [ "$SKIP_BUILD" != "1" ]; then
    echo "Building the Seed client in Release mode"
    cargo build --release --locked --features=runtime-benchmarks
else
    echo "Skipping building seed client..."
fi

populate_pallet_list
run_benchmark

