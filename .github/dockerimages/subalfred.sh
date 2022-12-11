# Build Seed
$HOME/.cargo/bin/cargo build --locked --release

# Get and compile Subalfred
git clone https://github.com/hack-ink/subalfred.git && cd subalfred
$HOME/.cargo/bin/cargo build --locked --release

# Do a storage check and save the output inside the output folder
./target/release/subalfred check runtime --executable ../target/release/seed --chain dev --live https://porcini.au.rootnet.app/ --property storage | tee /output/storage_results.txt
./target/release/subalfred check runtime --executable ../target/release/seed --chain dev --live https://porcini.au.rootnet.app/ --property version | tee /output/version_results.txt