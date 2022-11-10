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
npx hardhat test
```
Note: currently requires running one test at a time with `.only()`, until a solution for testing with fresh state is found


## Formatting/Linting
```
make fmt
```
## Benchmarks
TODO!
