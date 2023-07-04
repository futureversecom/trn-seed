#!/bin/bash

. ./scripts/getoptions.sh
VERSION=0.1
inputs_arguments() {
    setup   REST help:usage -- "Usage: ./scripts/compare.sh [options]... [arguments]..." ''
    msg -- 'Options:'
    flag    PORCINI         -p  --porcini   init:=0             -- "Get Porcini's state"
    flag    ROOT            -r  --root      init:=0             -- "Get Root's state"
    disp    :usage  -h  --help
}
eval "$(getoptions inputs_arguments - "$0") exit 1"

SECOND_ENDPOINT="wss://porcini.au.rootnet.app/ws"
if [ "$PORCINI" = "1" ]; then
    SECOND_ENDPOINT="wss://porcini.au.rootnet.app/ws"
fi
if [ "$ROOT" = "1" ]; then
    SECOND_ENDPOINT="wss://root.rootnet.live/archive/ws"
fi


deno run --allow-net --allow-write  ./scripts/compare.ts   ws://127.0.0.1:9944  ${SECOND_ENDPOINT}

mkdir -p ./output
mv ./diff.txt ./output/diff.txt