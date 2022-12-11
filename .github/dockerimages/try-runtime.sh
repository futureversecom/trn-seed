# Build Seed
$HOME/.cargo/bin/cargo build --locked --release

# Do try-runtime
./target/release/seed try-runtime --chain dev on-runtime-upgrade live --uri wss://porcini.au.rootnet.app:443/archive/ws 2>&1 | tee /output/results.txt