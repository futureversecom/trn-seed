# Seed Integration Tests

[![CI](https://github.com/futureversecom/trn-seed/actions/workflows/ci.yml/badge.svg)](https://github.com/futureversecom/trn-seed/actions/workflows/ci.yml)

---

Note: The tests require a running instance of the seed node.

## Dependencies

Install yarn dependencies

```shell
yarn
```

Copy Env file

```shell
cp .env.example .env
```

## Running tests

### Local

- Specify `CONNECTION_TYPE=local` in the `.env` file.
- Run node locally

```sh
cargo run -- --dev --unsafe-ws-external --unsafe-rpc-external --rpc-cors=all
```

- Note: You need to run specific test files, since the suite will not handle startup and shutdown of local node (state is persisted across runs; manual node restarts required)

```sh
yarn hardhat test test/TestCallGasEstimates.test.ts
```

#### TODO

- [ ] Add support for support `ConnectionType` of `binary` in [node.ts](./node.ts) to handle startup and shutdown of local node
- [ ] Multi-process startup and graceful shutdown (each test suite should spin up its own node and be able to be run in parallel - like the docker approach below)

### Docker

The test suite will handle spinning up and down the local node (in docker) for it to test against.

#### Build image

```sh
docker build -t seed/pr -f Dockerfile .
```

#### Run tests

```shell
yarn test
```

## CI pipeline

Integration tests are run against the latest seed node/image built by the [Dockerfile](../Dockerfile) upon pull requests
to the `main` branch - in the Github actions CI pipeline.
