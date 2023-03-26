#!/bin/sh

list_pallets () {
    if [ "$LIST_ENABLED" = "0" ]; then
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
    fi
}

skip_pallet () {
    for EXCLUDED_PALLET in "${EXCLUDED_PALLETS[@]}"; do
        if [ "$EXCLUDED_PALLET" == "$1" ]; then
            echo 0
            exit 0
        fi
    done
    
    echo 1
}

display_usage () {
    echo "Usage:"
    echo "  tools.sh benchmark pallets [option]"
    echo ""
    echo "Description:"
    echo "  Runs substrate benchmarks to generate weight files inside the './output' folder."
    echo "  Default steps is set to 1. Default repeat is set to 1 "
    echo ""
    echo "Options: "
    echo "      -p string   Pallets to be benchmarked."
    echo "                  To benchmark all pallets use the star character '*'. If this flag is omitted by default it"
    echo "                  will benchmark all pallets"
    echo "                  Example: tools.sh benchmark pallets -p pallet_balances pallet_assets"
    echo "      -s string   Speed at which the pallets are benchmarked. Available speeds [normal, fast, warp]"
    echo "                  normal: 50 steps    20 repeat"
    echo "                  fast:   25 steps    10 repeat"
    echo "                  warp:   10 steps     4 repeat"
    echo "      -l          List all pallets that are available to benchmark"
    echo "      -t          Use the provided template to generate the weight files"
    echo "                  This is used when the pallet weight file is needed"
    echo "      --steps     How many steps to do (1 default)"
    echo "      --repeat    How many repeats to do (1 default)"
    echo "      --ci        Copies the benchmark results to the external '/output' folder"
    echo "      --help      Displays help message"
}

STEPS_READ=$(./scripts/misc/misc.sh arg_value --steps $@)
REPEAT_READ=$(./scripts/misc/misc.sh arg_value --repeat $@)
LIST_ENABLED=$(./scripts/misc/misc.sh arg_exists -l $@)
TEMPLATE_ENABLED=$(./scripts/misc/misc.sh arg_exists -t $@)
PALLETS=$(./scripts/misc/misc.sh arg_value -p $@)
SPEED=$(./scripts/misc/misc.sh arg_value -s $@)
OUTPUT_FOLDER="./output"
TEMPLATE_LOCATION="./scripts/pallet_template.hbs"
TEMPLATE_ARG=""
ERR_FILE="$OUTPUT_FOLDER/benchmarking_errors.txt"
HELP=$(./scripts/misc/misc.sh arg_exists --help $@)
CI=$(./scripts/misc/misc.sh arg_exists --ci $@)

if [ "$HELP" = "0" ]; then
    display_usage
    exit 0
fi

case "$SPEED" in
    normal)     STEPS=50; REPEAT=20;;
    fast)       STEPS=25; REPEAT=5;;
    warp)       STEPS=10; REPEAT=5;;
    "")         STEPS=1; REPEAT=1;;
    *)          echo "Error: unrecognized speed >>$SPEED<<"; display_usage; exit 0;;
esac

if [ ! -z "$STEPS_READ" ]; then
    STEPS=$STEPS_READ
fi
if [ ! -z "$REPEAT_READ" ]; then
    REPEAT=$REPEAT_READ
fi

if [ -z "$PALLETS" ]; then
    PALLETS="*"
    echo "'--pallets' flag not found. All pallets will be benchmarked."
fi

if [ "$TEMPLATE_ENABLED" = "0" ]; then
    TEMPLATE_ARG="--template $TEMPLATE_LOCATION"
fi

echo "Benchmark arguments: "
echo "  Steps:          $STEPS"
echo "  Repeat:         $REPEAT"
echo "  Pallets:        $PALLETS"
echo "  Use Template:   $TEMPLATE_ENABLED (0 is true)"
echo "  List pallets:   $LIST_ENABLED (0 is true)"

# Building binary
cargo build --release  --locked --features=runtime-benchmarks

# Create output folder
mkdir -p output

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

# Get all pallets
if [ "$PALLETS" = "*" ]; then
    PALLETS=($(./target/release/seed benchmark pallet --list --chain dev | tail -n+2 | cut -d',' -f1 | sort | uniq ))
fi

# List pallets available for benchmarking
if [ "$LIST_ENABLED" = "0" ]; then
    list_pallets
    exit 0
fi

# Delete the error file before each run.
rm -f $ERR_FILE

# Benchmark each pallet.
for PALLET in "${PALLETS[@]}"; do
    SKIP=$(skip_pallet $PALLET)
    if [ "$SKIP" = "0" ] ; then
        echo "[ ] Skipping pallet $PALLET";
        continue
    fi
    
    echo "[+] Benchmarking $PALLET";
    OUTPUT=$(./target/release/seed benchmark pallet --chain=dev --steps=$STEPS --repeat=$REPEAT --pallet="$PALLET" --extrinsic="*" --execution=wasm --wasm-execution=compiled --heap-pages=4096 $TEMPLATE_ARG --output $OUTPUT_FOLDER 2>&1 )
    if [ $? -ne 0 ]; then
        echo "$OUTPUT" >> "$ERR_FILE"
        echo "[-] Failed to benchmark $PALLET. Error written to $ERR_FILE; continuing..."
    fi
done

if [ "$CI" = "0" ]; then
    cp -r ./output/. /output
fi
    echo "  Default steps is set to 1. Default repeat is set to 1 "
    echo ""
    echo "Options: "
    echo "      -p string   Pallets to be benchmarked."
    echo "                  To benchmark all pallets use the star character '*'. If this flag is omitted by default it"
    echo "                  will benchmark all pallets"
    echo "                  Example: tools.sh benchmark pallets -p pallet_balances pallet_assets"
    echo "      -s string   Speed at which the pallets are benchmarked. Available speeds [normal, fast, warp]"
    echo "                  normal: 50 steps    20 repeat"
    echo "                  fast:   25 steps    10 repeat"
    echo "                  warp:   10 steps     4 repeat"
    echo "      -l          List all pallets that are available to benchmark"
    echo "      -t          Use the provided template to generate the weight files"
    echo "                  This is used when the pallet weight file is needed"
    echo "      --steps     How many steps to do (1 default)"
    echo "      --repeat    How many repeats to do (1 default)"
    echo "      --ci        Copies the benchmark results to the external '/output' folder"
    echo "      --help      Displays help message"
}

STEPS_READ=$(./scripts/misc/misc.sh arg_value --steps $@)
REPEAT_READ=$(./scripts/misc/misc.sh arg_value --repeat $@)
LIST_ENABLED=$(./scripts/misc/misc.sh arg_exists -l $@)
TEMPLATE_ENABLED=$(./scripts/misc/misc.sh arg_exists -t $@)
PALLETS=$(./scripts/misc/misc.sh arg_value -p $@)
SPEED=$(./scripts/misc/misc.sh arg_value -s $@)
OUTPUT_FOLDER="./output"
TEMPLATE_LOCATION="./scripts/pallet_template.hbs"
TEMPLATE_ARG=""
ERR_FILE="$OUTPUT_FOLDER/benchmarking_errors.txt"
HELP=$(./scripts/misc/misc.sh arg_exists --help $@)
CI=$(./scripts/misc/misc.sh arg_exists --ci $@)

if [ "$HELP" = "0" ]; then
    display_usage
    exit 0
fi

case "$SPEED" in
    normal)     STEPS=50; REPEAT=20;;
    fast)       STEPS=25; REPEAT=5;;
    warp)       STEPS=10; REPEAT=5;;
    "")         STEPS=1; REPEAT=1;;
    *)          echo "Error: unrecognized speed >>$SPEED<<"; display_usage; exit 0;;
esac

if [ ! -z "$STEPS_READ" ]; then
    STEPS=$STEPS_READ
fi
if [ ! -z "$REPEAT_READ" ]; then
    REPEAT=$REPEAT_READ
fi

if [ -z "$PALLETS" ]; then
    PALLETS="*"
    echo "'--pallets' flag not found. All pallets will be benchmarked."
fi

if [ "$TEMPLATE_ENABLED" = "0" ]; then
    TEMPLATE_ARG="--template $TEMPLATE_LOCATION"
fi

echo "Benchmark arguments: "
echo "  Steps:          $STEPS"
echo "  Repeat:         $REPEAT"
echo "  Pallets:        $PALLETS"
echo "  Use Template:   $TEMPLATE_ENABLED (0 is true)"
echo "  List pallets:   $LIST_ENABLED (0 is true)"

# Building binary
cargo build --release  --locked --features=runtime-benchmarks

# Create output folder
mkdir -p output

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

# Get all pallets
if [ "$PALLETS" = "*" ]; then
    PALLETS=($(./target/release/seed benchmark pallet --list --chain dev | tail -n+2 | cut -d',' -f1 | sort | uniq ))
fi

# List pallets available for benchmarking
if [ "$LIST_ENABLED" = "0" ]; then
    list_pallets
    exit 0
fi

# Delete the error file before each run.
rm -f $ERR_FILE

# Benchmark each pallet.
for PALLET in "${PALLETS[@]}"; do
    SKIP=$(skip_pallet $PALLET)
    if [ "$SKIP" = "0" ] ; then
        echo "[ ] Skipping pallet $PALLET";
        continue
    fi
    
    echo "[+] Benchmarking $PALLET";
    OUTPUT=$(./target/release/seed benchmark pallet --chain=dev --steps=$STEPS --repeat=$REPEAT --pallet="$PALLET" --extrinsic="*" --execution=wasm --wasm-execution=compiled --heap-pages=4096 $TEMPLATE_ARG --output $OUTPUT_FOLDER 2>&1 )
    if [ $? -ne 0 ]; then
        echo "$OUTPUT" >> "$ERR_FILE"
        echo "[-] Failed to benchmark $PALLET. Error written to $ERR_FILE; continuing..."
    fi
done

if [ "$CI" = "0" ]; then
    cp -r ./output/. /output
fi
