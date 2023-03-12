#!/bin/bash
mkdir -p output

./ci-scripts/try-runtime.sh

rm -r subalfred
