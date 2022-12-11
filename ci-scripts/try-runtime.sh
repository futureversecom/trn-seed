# Build Seed
cargo build --locked --release --features try-runtime

# Do try-runtime
./target/release/seed try-runtime --chain dev on-runtime-upgrade live --uri wss://porcini.au.rootnet.app:443/archive/ws 2>&1 | tee /output/try_runtime_results.txt
