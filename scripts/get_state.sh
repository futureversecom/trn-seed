#!/bin/bash

. ./scripts/getoptions.sh
VERSION=0.2
inputs_arguments() {
    setup   REST help:usage -- "Usage: ./scripts/get_state.sh [options]... [arguments]..." ''
    msg -- 'Options:'
    flag    SKIP_DEP        -d              init:=0             -- "Skips installing dependencies"
    flag    PORCINI         -p  --porcini   init:=0             -- "Get Porcini's state"
    flag    ROOT            -r  --root      init:=0             -- "Get Root's state"
    disp    :usage  -h  --help
    disp    VERSION     --version
}
eval "$(getoptions inputs_arguments - "$0") exit 1"

# Check if python is installed
if ! [[ "$(python3 -V)" =~ "Python 3" ]]; then
    echo "python3 is not installed, please install it."
    exit 1
fi

if ! [ "$SKIP_DEP" = "1" ]; then
    # Install dependenices
    python3 -m venv ./scripts/penv
    source ./scripts/penv/bin/activate
    echo "Installing Python dependencies"
    pip install -r ./scripts/requirements.txt > /dev/null 2>&1
fi

CONFIG="./scripts/networks/porcini.yaml"
if [ "$PORCINI" = "1" ]; then
    CONFIG="./scripts/networks/porcini.yaml"
fi
if [ "$ROOT" = "1" ]; then
    CONFIG="./scripts/networks/root.yaml"
fi

python3 ./scripts/get_state.py --config "${CONFIG}"

cp ./target/release/seed ./output/binary