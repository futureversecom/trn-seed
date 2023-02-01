#!/bin/bash

# This is a simple bash script that allows us to do two things:
#   1) Display pallets that can be benchmarked
#   2) Benchmark pallets
#
# If 2) option is selected then we need to choose what pallets we want to bench (all or specific ones)
# and the speed of running those benchmarks. The speeds are:
# I)    Normal      (50s,   20r)
# II)   Fast        (25s,   5r)
# III)  Lightspeed  (2s,    1r)
#
# The default behavior is to start with a menu but this can be override if at least one flag
# is supplied. Supported flags:
# -f -> Fast execution mode
# -q -> Lightspeed execution mode
# -l -> Display pallets that can be benchmarked
# -p "pallet_balance pallet_assets" -> Selects which pallets to benchmark

set -e

# Default vaules
STEPS=50
REPEAT=20
OUTPUT_FOLDER="./output"
PALLET="*"
MODE="release"
CHAIN="dev"

EXECUTION_MODE="Normal"
LIST_PALLETS=false
USE_TEMPLATE=false
TEMPLATE_LOCATION="./scripts/pallet_template.hbs"
TEMPLATE_ARG=""

if [ $# -eq 0 ]; then
    echo "Select modus operandi: "
    select opt in List-Pallets Benchmark-Pallets; do
        case $opt in
            List-Pallets)
                LIST_PALLETS=true
                break
            ;;
            Benchmark-Pallets)
                LIST_PALLETS=false
                break
            ;;
            *)
                echo "Invalid option $REPLY"
            ;;
        esac
    done
    
    if ! "$LIST_PALLETS"; then
        echo "Exeuction speed:"
        select opt in Normal Fast Lightspeed; do
            case $opt in
                Normal)
                    EXECUTION_MODE="Normal"
                    break
                ;;
                Fast)
                    EXECUTION_MODE="Fast"
                    break
                ;;
                Lightspeed)
                    EXECUTION_MODE="Lightspeed"
                    break
                ;;
                *)
                    echo "Invalid option $REPLY"
                ;;
            esac
        done
        
        echo "Benchmark all pallets?"
        select opt in Yes No; do
            case $opt in
                Yes)
                    PALLET="*"
                    break
                ;;
                No)
                    read -r -p "Pallet names: " PALLET
                    break
                ;;
                *)
                    echo "Invalid option $REPLY"
                ;;
            esac
        done
        
        echo "Do you want to generate weights for runtime or generate weightinfo for pallet?"
        select opt in Runtime-Weights Pallet-WeightInfo; do
            case $opt in
                Runtime-Weights)
                    USE_TEMPLATE=false
                    break
                ;;
                Pallet-WeightInfo)
                    USE_TEMPLATE=true
                    break
                ;;
                *)
                    echo "Invalid option $REPLY"
                ;;
            esac
        done
    fi
fi


# Read flags
while getopts dqfblp:t flag
do
    case "${flag}" in
        f) EXECUTION_MODE="Fast";;
        q) EXECUTION_MODE="Lightspeed";;
        p) PALLET=${OPTARG};;
        l) LIST_PALLETS=true;;
        t) USE_TEMPLATE=true;;
    esac
done


if [ "$EXECUTION_MODE" = "Fast" ]; then
    STEPS=25
    REPEAT=5
fi

if [ "$EXECUTION_MODE" = "Lightspeed" ]; then
    STEPS=2
    REPEAT=1
fi

if "$USE_TEMPLATE"; then
    TEMPLATE_ARG="--template $TEMPLATE_LOCATION"
fi

START_TIMER_1=$(date +%s)

echo "Chain: $CHAIN"
echo "Output folder: $OUTPUT_FOLDER"
echo "Steps: $STEPS"
echo "Repeat: $REPEAT"
echo "Pallet: $PALLET"

START_TIMER_2=$(date +%s)
echo "Building the Seed client in Release mode"
cargo build --release  --locked --features=runtime-benchmarks # This is acctualy supposed to be production and not release
END_TIMER_2=$(date +%s)


# Manually exclude some pallets.
EXCLUDED_PALLETS=(
    # Helper pallets
    "pallet_election_provider_support_benchmarking"
    # Pallets without automatic benchmarking
    "pallet_babe"
    "pallet_grandpa"
    "pallet_mmr"
    "pallet_offences"
)

if [ "$PALLET" = "*" ]; then
    PALLETS=($(./target/$MODE/seed benchmark pallet --list --chain $CHAIN | tail -n+2 | cut -d',' -f1 | sort | uniq ))
else
    PALLETS=($PALLET)
fi

if [ "$OUTPUT_FOLDER" = "./output" ]; then
    mkdir -p output
fi

if "$LIST_PALLETS"; then
    for PALLET in "${PALLETS[@]}"; do
        NOT_SKIP=true
        for EXCLUDED_PALLET in "${EXCLUDED_PALLETS[@]}"; do
            if [ "$EXCLUDED_PALLET" == "$PALLET" ]; then
                NOT_SKIP=false
                break
            fi
        done
        if $NOT_SKIP; then
            echo "$PALLET";
        fi
    done
    exit 0;
fi

ERR_FILE="$OUTPUT_FOLDER/benchmarking_errors.txt"
# Delete the error file before each run.
rm -f $ERR_FILE

START_TIMER_3=$(date +%s)
# Benchmark each pallet.
for PALLET in "${PALLETS[@]}"; do
    SKIP=false
    for EXCLUDED_PALLET in "${EXCLUDED_PALLETS[@]}"; do
        if [ "$EXCLUDED_PALLET" == "$PALLET" ]; then
            SKIP=true
            break
        fi
    done
    
    if $SKIP; then
        echo "[ ] Skipping pallet $PALLET";
        continue
    fi
    
    echo "[+] Benchmarking $PALLET";
    
    OUTPUT=$(./target/$MODE/seed benchmark pallet --chain=$CHAIN --steps=$STEPS --repeat=$REPEAT --pallet="$PALLET" --extrinsic="*" --execution=wasm --wasm-execution=compiled --heap-pages=4096 $TEMPLATE_ARG --output $OUTPUT_FOLDER 2>&1 )
    if [ $? -ne 0 ]; then
        echo "$OUTPUT" >> "$ERR_FILE"
        echo "[-] Failed to benchmark $PALLET. Error written to $ERR_FILE; continuing..."
    fi
done
END_TIMER_3=$(date +%s)
END_TIMER_1=$(date +%s)



secs=$(($END_TIMER_1-$START_TIMER_1))
printf 'Total Elapsed Time: %02dh:%02dm:%02ds\n' $((secs/3600)) $((secs%3600/60)) $((secs%60))
secs=$(($END_TIMER_2-$START_TIMER_2))
printf 'Binary Build Elapsed Time: %02dh:%02dm:%02ds\n' $((secs/3600)) $((secs%3600/60)) $((secs%60))
secs=$(($END_TIMER_3-$START_TIMER_3))
printf 'Benchmark Exeuction Elapsed Time: %02dh:%02dm:%02ds\n' $((secs/3600)) $((secs%3600/60)) $((secs%60))
