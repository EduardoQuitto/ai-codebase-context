# Development setup

Requirements: Rust stable ≥ 1.97 (MSRV), Git. No Node/Python/DB needed.

```sh
rustup update stable
cargo build
cargo test --workspace
./target/debug/aicc --help
```

Useful env:

```sh
RUST_LOG=debug ./target/debug/aicc status --path ./examples/basic-project
./target/debug/aicc status --json --path ./examples/basic-project
```

Windows, macOS and Linux are first-class; avoid shell-outs and hardcoded separators.
