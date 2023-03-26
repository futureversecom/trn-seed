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
### Run Porcini Fork
This script fetches Porcini storage data, builds a new chain specification using that storage data and runs a node with it.

#### Locally
```bash
./scripts/tools.sh storage fetch --run
```
#### Using Podman/Docker Compose
```bash
# Docker Compose
sudo docker-compose -f ./scripts/compose.yaml up fork-porcini-and-run-it
# Or Podman Compose
podman-compose -f ./scripts/compose.yaml up fork-porcini-and-run-it
```

### Run Full-test
This script does the following:
    1) Checks the storage and version differences between the active branch and Porcini/Root
    2) Fetches Porcini/Root storage, builds a chain specification out of it and runs a local node with it
    3) Changes specification version to 100 and runs a runtime upgrade
    4) Once the runtime upgrade is done, it fetches local node's storage and builds a chain specification from it
    5) Stores the chain specification and storage difference between forked chain and upgraded chain

#### Locally
```bash
./scripts/tools.sh full-test
# For the version which stops the container once everything is done:
# ./scripts/tools.sh full-test --no-wait
```
#### Using Podman/Docker Compose
```bash
# Docker Compose
sudo docker-compose -f ./scripts/compose.yaml up full-test
# For the version which stops the container once everything is done:
# sudo docker-compose -f ./scripts/compose.yaml up full-test-no-wait

# Or Podman Compose
podman-compose -f ./scripts/compose.yaml up full-test
# For the version which stops the container once everything is done:
# podman-compose -f ./scripts/compose.yaml up full-test-no-wait
```

### Run Runtime Upgrade
This builds a wasm file and runs a runtime upgrade with it. You must have a node running for this to work. See `Run Porcini Pork`

#### Locally
```bash
./scripts/tools.sh runtime upgrade
```