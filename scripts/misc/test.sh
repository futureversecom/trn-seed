#!/bin/sh
set -e

KillJobs() {
    for job in $(jobs -p); do
        kill -s SIGTERM $job > /dev/null 2>&1 || (sleep 10 && kill -9 $job > /dev/null 2>&1 &)
        
    done
}
TrapQuit() {
    KillJobs
}

# trap "exit" INT TERM
# trap "kill 0" EXIT
trap TrapQuit EXIT

HELP=$(./scripts/misc/misc.sh arg_exists --help $@)
if [ "$HELP" = "0" ]; then
    echo "Usage:"
    echo "  tools.sh full-test [option]"
    echo ""
    echo "Description:"
    echo "  This script does the following:"
    echo "    1) Checks the storage and version differences between the active branch and Porcini/Root"
    echo "    2) Fetches Porcini/Root storage, builds a chain specification out of it and runs a local node with it"
    echo "    3) Changes specification version to 100 and runs a runtime upgrade"
    echo "    4) Once the runtime upgrade is done, it fetches local node's storage and builds a chain specification from it"
    echo "    5) Stores the chain specification and storage difference between forked chain and upgraded chain"
    echo ""
    echo "  By default the upgraded chain doesn't stop once all the steps are done. To stop everything after the"
    echo "  last step is done pass the '--no-wait' flag"
    echo ""
    echo "Options: "
    echo "      --porcini       Scraps Porcini Network (default)"
    echo "      --root          Scraps Root Network"
    echo "      --no-wait       Stops the upgraded node from running once all steps are done"
    echo "      --ci            All fetched and generated data will be copied to the external '/output' folder"
    echo "      --help          Display this usage message"
    exit 0
fi

CI=$(./scripts/misc/misc.sh arg_exists --ci $@)
ROOT=$(./scripts/misc/misc.sh arg_exists --root $@)
NO_WAIT=$(./scripts/misc/misc.sh arg_exists --no-wait $@)
ORG_SPEC_VERSION=$(grep "spec_version" ./runtime/src/lib.rs)
CI_FLAG=""
ROOT_FLAG=""

if [ "$CI" = "0" ]; then
    CI_FLAG="--ci"
fi
if [ "$ROOT" = "0" ]; then
    ROOT_FLAG="--root"
fi

echo "--------------------------------------"
echo "Operation:    Full-test               "
echo "CI enabled:   $CI (0 is true)         "
echo "No Wait:      $NO_WAIT (0 is true)    "

echo "Executing storage check #1/8"
./scripts/tools.sh storage check "$CI_FLAG $ROOT_FLAG"

echo "Scraping local storage #2/8"
./scripts/tools.sh storage fetch "$CI_FLAG $ROOT_FLAG"

echo "Running a node in the background #3/8"
./target/release/seed --chain ./output/fork.json --alice --force-authoring --tmp --rpc-cors=all --unsafe-rpc-external --unsafe-ws-external --rpc-methods unsafe &

# Change spec verion to 100 to trigger the runtime upgrade
sed 's/.*spec_version.*/    spec_version: 100,/' ./runtime/src/lib.rs -i

echo "Waiting for the node to start running. 30 seconds #4/8"
sleep 30

echo "Runing runtime upgrade #5/8"
./scripts/tools.sh runtime upgrade "$CI_FLAG"

echo "Waiting for the node to reorganize itself after a runtime upgrade. 10 seconds #6/8"
sleep 10

echo "Fetching local storage #7/8"
./scripts/tools.sh storage fetch "$CI_FLAG" --local --file-prefix after_runtime_upgrade_

echo "Creating diff #8/8"
git diff --no-index ./output/fork.json ./output/after_runtime_upgrade_fork.json > ./output/fork.diff || true

if [ "$CI" = "0" ]; then
    cp ./output/fork.diff /output/fork.diff
fi

# Return spec version back
sed "s/.*spec_version.*/$ORG_SPEC_VERSION/" ./runtime/src/lib.rs -i

echo "--------------"
echo "  All Done :D "
echo "--------------"

if [ ! "$NO_WAIT" = "0" ]; then
    wait
fi
