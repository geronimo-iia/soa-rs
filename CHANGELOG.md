# Changelog

All notable changes to this project will be documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `Soa::retain` and `Soa::retain_mut` — in-place compaction with a predicate,
  matching `Vec::retain` / `Vec::retain_mut`.
- `Soa::dedup_by`, `Soa::dedup`, `Soa::dedup_by_key` — remove consecutive
  duplicate elements, matching the `Vec` API.
- `Soa::drain` — yields owned elements from a range, closing the gap on drop;
  returns a `Drain` iterator (`ExactSizeIterator` + `FusedIterator`).
- `Soa::split_off` — splits the collection at an index, returning the tail as a
  new `Soa` without cloning.
- `Soa::resize` and `Soa::resize_with` — grow or shrink to a target length,
  matching `Vec::resize` / `Vec::resize_with`.
- `Soa::sort_indices_by` and `Soa::sort_indices_by_key` — return a sorted
  `Vec<usize>` index vector without moving data; the idiomatic pattern for
  render-depth ordering and ECS iteration in sorted order.
- `ChunksExact` now implements `DoubleEndedIterator` (`.rev()` support),
  `ExactSizeIterator` (`.len()`), and `FusedIterator` — matching `std::slice::ChunksExact`.
- CI workflow (GitHub Actions): fmt, clippy, tests, `cargo audit` on every push and PR.
- Dependabot for weekly Cargo and Actions updates.
- `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `audit.toml` for consistent toolchain pinning.
- `CONTRIBUTING.md`, `SECURITY.md`, `THIRD_PARTY_NOTICES`, `AGENT.md`.

### Changed
- `soa![elem; N]` now calls `Soa::with_capacity(N)` upfront — single allocation
  instead of two for any `N > 4`.
- `Soa::truncate` no longer calls `pop` in a loop; drops elements in-place and
  sets `len` directly. For `!needs_drop` types the body reduces to a single store.
- `Soa::append` now uses a bulk `copy_to` per field array instead of an
  element-by-element `push` loop — significantly faster for large collections.
- `Soa::deserialize` (serde feature) pre-allocates via `seq.size_hint()`,
  eliminating redundant reallocations for formats that provide a length upfront
  (JSON, bincode, etc.).

## [1.0.0] — upstream baseline

This fork is based on [soa-rs 1.0.0](https://github.com/tim-harding/soa-rs)
by Timothy Harding. See the upstream repository for the full history prior to
this fork.
