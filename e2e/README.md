# Seed Integration Tests

[![CI](https://github.com/futureversecom/seed-integration-tests/actions/workflows/ci.yml/badge.svg)](https://github.com/futureversecom/seed-integration-tests/actions/workflows/ci.yml)

---

The tests require a running instance of the seed node.

Note: The easiest way to get one is to use the docker-compose file in the root of the project via `yarn seed`

- Pulling the image will require a [login to the docker registry](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry#authenticating-to-the-container-registry).
  - `read:packages` permission required for generating github token (classic) access token

## Dependencies

Install yarn dependencies

```shell
yarn
```

## Running tests

The test suite will handle spinning up and down the local node for it to test against. To run the tests:

Run seed node (via `docker-compose.yml`):

```shell
yarn seed
```

Run hardhat tests:

```shell
yarn test
```

## CI pipeline

Integration tests are run against the latest seed node/image (`ghcr.io/futureversecom/seed:latest`) upon pull requests to the `main` branch - in the Github actions CI pipeline.
