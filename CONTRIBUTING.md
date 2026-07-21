# Contributing to soa-rs

## Prerequisites

- Rust 1.87.0 — pinned in `.tool-versions` and `rust-toolchain.toml`.
  Install via [asdf](https://asdf-vm.com/) or [rustup](https://rustup.rs/).
- `git`

## Build and Test

```bash
cargo build --workspace                          # debug build
cargo test --workspace                           # all tests (default features)
cargo test --workspace --features serde          # include serde round-trip tests
cargo clippy --workspace -- -D warnings          # lint — must pass with zero warnings
cargo clippy --workspace --features serde -- -D warnings
cargo fmt --all -- --check                       # check formatting
cargo fmt --all                                  # auto-format
cargo audit                                      # security audit (requires cargo-audit)
```

## Feature Flags

| Flag    | What it enables                                      |
|---------|------------------------------------------------------|
| `serde` | `Serialize`/`Deserialize` on `Soa<T>` via `serde_json` |

Default features are empty — `serde` is opt-in.

## Project Layout

| Path | Purpose |
|---|---|
| `src/` | Main library (`no_std`) |
| `soa-rs-derive/` | Proc macro crate (`#[derive(Soars)]`) |
| `soa-rs-testing/` | Integration tests and benchmarks |
| `plans/` | Implementation plans for upcoming features |

## Adding a Feature

1. Check `plans/` for an existing plan or write one first.
2. Implement in the correct module — see `AGENT.md` for the module map.
3. Add doc-tests on the new public methods.
4. Add integration tests in `soa-rs-testing/src/lib.rs`.
5. Run the full check:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace -- -D warnings
   cargo clippy --workspace --features serde -- -D warnings
   cargo test --workspace
   cargo test --workspace --features serde
   cargo doc --no-deps
   ```

## Unsafe Code

All `unsafe` blocks must have a `// SAFETY:` comment explaining which
invariants are upheld. See `src/soa_raw.rs` for the contract each
`SoaRaw` method must satisfy.

## Benchmarks

```bash
cargo bench --manifest-path soa-rs-testing/Cargo.toml
```

## Release Process

1. Update version in `Cargo.toml` and `soa-rs-derive/Cargo.toml`.
2. Update `CHANGELOG.md`.
3. Push a `vX.Y.Z` tag — the release workflow publishes both crates to
   crates.io automatically (derive first, then the main crate).
