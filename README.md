# Seed

The seed chain is a precursor chain for bootstrapping the futureverse root network

## Building
Run `cargo build --release`

## Testing
Unit tests
Run `cargo test`
E2E tests
TODO!

## Formatting/Linting
Run `make fmt`
## Try Runtime
Try-runtime allows testing new changes of the node against live versions of the network that contain the current state, helping build assurance against breaking changes to storage or other components.

1. Build the node with `try-runtime` enabled: `cargo build --release --features try-runtime`
2. Use try-runtime subcommands: ./target/release/seed try-runtime -h
## Benchmarks
TODO!