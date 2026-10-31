# Building

You need Rust through [rustup](https://rustup.rs/). `rust-toolchain.toml`
selects the stable toolchain with `clippy` and `rustfmt`.

## Build and test

These are the checks CI runs on Windows and Linux:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

The program is `target/release/tf2-queue-query.exe` on Windows and
`target/release/tf2-queue-query` on Linux.

To check the Linux code from Windows without a Linux machine:

```text
rustup target add x86_64-unknown-linux-gnu
cargo clippy --all-targets --locked --target x86_64-unknown-linux-gnu -- -D warnings
```

## Releasing

1. Set the new version in `Cargo.toml` and add it to `CHANGELOG.md`.
2. Commit, then tag the commit with the same version, for example `v0.2.0`,
   and push the tag.
3. The Release workflow checks that the tag matches `Cargo.toml`, builds both
   platforms, and creates a draft GitHub release with
   `tf2-queue-query-v<version>-windows-x86_64.zip`,
   `tf2-queue-query-v<version>-linux-x86_64.tar.gz`, and a `.sha256` file for
   each. Each archive contains the program, `README.md`, `LICENSE`,
   `CHANGELOG.md`, and `docs/`.
4. Review the draft and publish it.

Running the Release workflow by hand builds the same archives as workflow
artifacts without creating a release.
