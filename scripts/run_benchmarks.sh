#!/bin/bash

. ./scripts/getoptions.sh

VERSION=0.1
inputs_arguments() {
    setup   REST help:usage -- "Usage: ./scripts/run_benchmark.sh [options]... [arguments]..." ''
    msg -- 'Options:'
    param   TEMPLATE_PATH           --template      init:="./scripts/pallet_template.hbs"   -- "Specifies template location"
    param   OUTPUT_FOLDER       -o  --output        init:="./runtime/src/weights"                        -- "Folder where all the weight files will be stored"
    param   PALLETS             -p  --pallets       init:="*"                               -- "List of pallets that need to be bechmarked. Default is all. Example: -p \"pallet_nft pallet_echo\""
    param   STEPS               -s  --steps         init:=50                                -- "How many steps to do. Default is 50"
    param   REPEAT              -r  --repeat        init:=20                                -- "How many repeats to do. Default is 20"
    flag    USE_TEMPLATE        -t                                                          -- "If set then the template will be used to generate the weight files"
    flag    JUST_CUSTOM_PALLETS -c                                                          -- "Benchmarks just our own custom pallets"
    param   BINARY_LOCATION     -b                  init:="./target/release/seed"           -- "Path where the binary is located"
    flag    LIST_PALLET         -l                                                          -- "List all pallets that can be benchmarked"
    disp    :usage  -h --help
    disp    VERSION    --version
}

run_benchmark() {
    echo "Pallets: ${PALLETS[@]}"
    echo "Steps: $STEPS, Repeat: $REPEAT"
    
    if [ "$LIST_PALLET" = 1 ]; then
        exit 0
    fi
    
    rm -f $ERR_FILE
    mkdir -p "$OUTPUT_FOLDER"
    
    for PALLET in "${PALLETS[@]}"; do
        if is_pallet_excluded; then
            echo "[ ] Skipping pallet $PALLET";
            continue
        fi
        
        FILE_NAME="$PALLET.rs"
        TEMPLATE_NAME="${PALLET}_weights.rs"
        
        if [ "$USE_TEMPLATE" = "1" ]; then
            FILE_NAME="$TEMPLATE_NAME"
            TEMPLATE_ARG="--template $TEMPLATE_PATH";
        fi
        
        benchmark "$TEMPLATE_ARG" "$FILE_NAME"
        
        if is_custom_pallet && [ ! "$USE_TEMPLATE" = "1" ]; then
            benchmark "--template $TEMPLATE_PATH" "$TEMPLATE_NAME"
        fi
        
    done
}

benchmark() {
    set -x
    echo "[+] Benchmarking $PALLET";
    WEIGHT_FILENAME=$(echo $2 | tr '-' '_');
    OUTPUT=$($BINARY_LOCATION benchmark pallet --chain=dev --steps=$STEPS --repeat=$REPEAT --pallet="$PALLET" --extrinsic="*" --execution=wasm --wasm-execution=compiled --heap-pages=4096 --output "$OUTPUT_FOLDER/$WEIGHT_FILENAME" $1 2>&1 )
    if [ $? -ne 0 ]; then
        echo "$OUTPUT" >> "$ERR_FILE"
        echo "[-] Failed to benchmark $PALLET. Error written to $ERR_FILE; continuing..."
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
        "pallet_election_provider_support_benchmarking"
        # Pallets without automatic benchmarking
        "pallet_babe"
        "pallet_grandpa"
        "pallet_mmr"
        "pallet_offences"
        # pallet taking too long!
        "pallet_assets"
        "frame_benchmarking"
        "pallet_election_provider_multi_phase"
        "pallet_dex_weights"
		"pallet_echo_weights"
		"pallet_im_online"
        "pallet_dex_weights"
        "pallet_echo_weights"
        "pallet_erc20_peg_weights"
        "pallet_evm_chain_id_weights"
        "pallet_fee_control_weights"
        "pallet_futurepass_weights"
        "pallet_nft_peg_weights"
        "pallet_nft_weights"
        "pallet_proxy"
        "pallet_recovery"
        "pallet_sft_weights"
        "pallet_token_approvals_weights"
        "pallet_xls20_weights"
        "pallet_xrpl_bridge_weights"
    )
    
    CUSTOM_PALLETS=()
    for f in ./pallet/*/Cargo.toml; do
        pallet_name=$(awk -F' = ' '$1 == "name" {print $2}' $f | tr -d '"' | tr '-' '_')
        CUSTOM_PALLETS+=($pallet_name)
    done;
    
    if ! [ "$PALLETS" = "*" ]; then
        PALLETS=($PALLETS)
    fi
    if [ "$LIST_PALLET" = "1" ] || [ "$PALLETS" = "*" ]; then
        PALLETS=($($BINARY_LOCATION benchmark pallet --list --chain=dev | tail -n+2 | cut -d',' -f1 | sort | uniq ))
    fi
    if [ "$JUST_CUSTOM_PALLETS" = "1" ]; then
        PALLETS=("${CUSTOM_PALLETS[@]}")
    fi
}

eval "$(getoptions inputs_arguments - "$0") exit 1"

ERR_FILE="$OUTPUT_FOLDER/benchmarking_errors.txt"

# echo "Building the Seed client in Release mode"
# cargo build --release --locked --features=runtime-benchmarks

populate_pallet_list
run_benchmark

