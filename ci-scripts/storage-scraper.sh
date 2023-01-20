# Build Seed
cargo build --locked --release

# Copy binary and chain spec
mkdir ./ci-scripts/storage-scraper/data
cp ./target/release/seed ./ci-scripts/storage-scraper/data/binary
cp ./chain-spec/* ./ci-scripts/storage-scraper/data/

cd ./ci-scripts/storage-scraper
npm i
npm start

cd ../../

cp ./ci-scripts/storage-scraper/data/fork.json ./output/