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

1. Set the new version in `Cargo.toml`, run `cargo build` so `Cargo.lock`
   picks it up, and add the version to `CHANGELOG.md`.
2. Run the checks above. They use `--locked`, as the workflow does, so they
   fail if `Cargo.lock` was not updated.
3. Commit `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md`, then tag the commit
   with the same version, for example `v0.2.0`, and push the tag.
4. The Release workflow checks that the tag matches `Cargo.toml`, builds both
   platforms, and creates a draft GitHub release with
   `tf2-queue-query-v<version>-windows-x86_64.zip`,
   `tf2-queue-query-v<version>-linux-x86_64.tar.gz`, and a `.sha256` file for
   each. Each archive contains the program, `README.md`, `LICENSE`,
   `CHANGELOG.md`, and `docs/`.
5. Review the draft and publish it.

Running the Release workflow by hand, on any branch or tag, builds the same
archives as workflow artifacts without creating a release.
