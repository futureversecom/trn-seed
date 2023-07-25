# WARNING
Please refrain from merging this branch at the moment.

This branch is derived from a previous version of the repository through a fork. Its purpose was to demonstrate the configuration and usage of the XCMv3 crosschain protocol. 

The primary modifications include upgrading the substrate version dependency from 0.9.27 to 0.9.42, introducing XCM-related pallets and configurations, and disabling certain RPC features that are not the primary focus of this proof of concept. 

It is not necessary to merge this branch unless the relay is ready. 
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
