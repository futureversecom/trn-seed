# Seed

The seed chain is a precursor chain for bootstrapping the futureverse root network

## Building
```
cargo build --release
```

## Running
To run the project locally, first build the code, then run
```shell
./target/release/seed --dev
```

## Development

### Getting the right toolchain
To get the right toolchain execute the following command:
```shell
rustup show
```

## Testing
To test the project, run unit and E2E tests

Unit tests
```shell 
cargo test
```
E2E tests
Start the node, then run:

```shell
cd test-ts
yarn
yarn test
```

## Formatting/Linting
```
make fmt
```

## Benchmarks
See the [wiki](https://github.com/futureversecom/seed/wiki/How-to-benchmark)


## Scripts
### Run Porcini Pork
This script scraps Porcini, stores the scraped chain specification inside the `./output` folder and runs a chain with that scraped chain spec.

#### Locally
```bash
./scripts/run_porcini_fork.sh
```
#### Using Docker
```bash
mkdir -p output
podman build -t tporcini -f ./dockerimages/porcini-fork.Dockerfile .
podman run --rm -p "9933:9933" -p "9944:9944" -p "30333:30333" -v ./output:/output -t tporcini
```

### Run Runtime Upgrade
This builds a wasm file and runs a runtime upgrade with it. You must have a node running for this to work. See `Run Porcini Pork`

#### Locally
```bash
./scripts/run_runtime_upgrade.sh
```