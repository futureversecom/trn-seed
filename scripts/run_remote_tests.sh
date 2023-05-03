#!/bin/bash

FILE="$(eval pwd)"
FILE="${FILE}/snap.top"
if [ ! -f "$FILE" ]; then
    echo "Snapshot not found. Make sure you get it from here: https://centralitylimited-my.sharepoint.com/:u:/g/personal/marko_petrlic_centrality_ai/EbMeapJ8SfNJgLlTWv0ssDgBFjnvUjSumFMlWlZCfLyI8g?e=Z0FBVg"
    echo "Saved it as snap.top"
    exit 1
fi

SNAP="$FILE" cargo test --lib --package=seed-runtime --features=try-runtime --tests run_migrations
