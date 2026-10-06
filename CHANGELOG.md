# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Added

- An `opencl` feature and `--opencl` flag: a Bitcoin prefix search for OpenCL
  GPUs using batched affine additions, the secp256k1 endomorphism, and
  HASH160 range matching, with CPU re-verification of every hit (about
  1.5 billion keys/s on a Radeon 8060S).
- Bitcoin, Ethereum, and Solana exact-pattern GPU search.
- CPU, GPU, Hybrid, and deterministic Auto backend selection.
- Configurable GPU batch size and 1-100% duty-cycle limiting.
- A responsive 70% Hybrid default with short throttled dispatch windows.
- CPU/GPU correctness, result-race, resource-limit, and benchmark coverage.
- A user manual, security policy, contribution guide, and CI documentation
  build.

### Changed

- CPU matching compiles pattern state once and reuses the first generated batch.
- Solana GPU candidates reuse one SHA-512 digest.
- Solana support uses the focused `solana-keypair` crate instead of the full
  SDK.
- Stable Rust and SHA-3 dependencies replace prerelease versions.

### Fixed

- GPU winner publication is atomic across workgroups.
- Explicit GPU requests no longer silently fall back to CPU.
- Generation and output failures return a nonzero CLI exit status.
- Newly created Unix output files request owner-only permissions.
