<p align="center">
    <img src="./.github/logo.png" height="96">
    <h3 align="center">The Root Network Seed 🌱</h3>
</p>

Implementation of [therootnetwork.com](https://therootnetwork.com/) node in Rust, based on the Substrate framework.

This repo contains runtimes for the Root Network Mainnet and Porcini (Testnet). For more specific guides on how to build applications, see the [docs](https://docs.rootnet.live)

## Building

### Build from source
First install Rust. You may need to add Cargo's bin directory to your PATH environment variable.

```bash
curl https://sh.rustup.rs -sSf | sh
```

If you already have Rust installed, make sure you're using the latest version by running:

```bash
rustup update
```

Build the client by cloning this repository and running the following commands from the root directory of the repo:

```bash
cargo build --release
```

## Networks

This repo supports runtimes for Root (mainnet), Porcini (testnet)

### Connect to Root Mainnet

```bash
./target/release/seed --chain=root
```

### Connect to Porcini Testnet

```bash
./target/release/seed --chain=porcini
```

### Run a network locally

To run the project locally, first build the code, then run

```bash
./target/release/seed --dev
```

## Development

### Getting the right toolchain

To get the right toolchain execute the following command:

```bash
rustup show
```

### Run Unit Tests

```bash
cargo test
```
### Run E2E Tests

Refer to the instruction [here](./e2e)

### Formatting & Linting
```
cargo fmt
```

### Benchmarks

See the [wiki](https://github.com/futureversecom/seed/wiki/How-to-benchmark)

## Provide Feedback

- [Start a Discussion](https://github.com/futureversecom/trn-seed/discussions) with a question, piece of feedback, or idea you want to share with the team.
- [Open an Issue](https://github.com/futureversecom/trn-seed/issues) if you believe you've encountered a bug that you want to flag for the team.