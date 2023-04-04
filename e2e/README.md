# Seed Integration Tests

[![CI](https://github.com/futureversecom/trn-seed/actions/workflows/ci.yml/badge.svg)](https://github.com/futureversecom/trn-seed/actions/workflows/ci.yml)

---

Note: The tests require a running instance of the seed node.

## Dependencies

Install yarn dependencies

```shell
yarn
```

## Running tests

The test suite will handle spinning up and down the local node for it to test against. To run the tests:

Run seed node (e.g. via `docker-compose.yml`):

```shell
yarn seed
```

Run hardhat tests:

```shell
yarn test
```

## CI pipeline

Integration tests are run against the latest seed node/image built by the [Dockerfile](../Dockerfile) upon pull requests
to the `main` branch - in the Github actions CI pipeline.
