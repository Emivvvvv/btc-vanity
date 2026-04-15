# Copilot instructions for `btc-vanity`

## Build, test, and lint commands

- Build default feature set (Bitcoin only): `cargo build`
- Build with a feature set used in CI:
  - `cargo build --features solana`
  - `cargo build --features ethereum`
  - `cargo build --features all`
- Lint (CI gate): `cargo clippy --all-features -- -D warnings`
- Run test suite:
  - Default: `cargo test`
  - All features: `cargo test --all-features`
  - Doc tests: `cargo test --doc --all-features`

Single-test examples:

- Single library/unit test by name: `cargo test --lib test_parse_cli_defaults_backend_to_hybrid`
- Single integration test: `cargo test --test integration_tests test_bitcoin_vanity_address_prefix`
- Single GPU integration test (when GPU feature is enabled): `cargo test --features "gpu ethereum solana" --test gpu_backend_tests test_gpu_regex_is_rejected_but_auto_regex_uses_cpu`

Notes:

- `tests/cli_tests.rs` invokes `./target/debug/btc-vanity`; ensure the debug binary exists before running isolated CLI tests (`cargo build` or `cargo test` first).
- CI runs clippy with `--all-features`, then build/test across feature matrix `["", "solana", "ethereum", "all"]`.

## High-level architecture

- `src/main.rs` is the CLI entrypoint. It parses arguments (`cli.rs` + `flags.rs`), supports either a single pattern or `--input-file`, and merges per-line file flags with CLI flags via `VanityFlags::unify`.
- The public API lives in `src/vanity_addr_generator/vanity_addr.rs` (`VanityAddr`, `VanitySearchOptions`, `VanityBackend`, `VanityMode`). This is the orchestration layer for CPU/GPU/hybrid/auto execution.
- Chain-specific behavior is abstracted by `VanityChain` in `src/vanity_addr_generator/chain.rs` and implemented by keypair types in `src/keys_and_address/*`:
  - validation rules (base58/base16, regex validation),
  - chain-specific pattern adjustments,
  - keypair/address construction from private keys (including GPU-derived keys).
- CPU search path is batch-oriented (`BATCH_SIZE = 256`) and uses optimized byte-comparison helpers in `src/vanity_addr_generator/comp.rs` for prefix/suffix/anywhere matching.
- GPU support is behind the `gpu` feature and implemented in `src/vanity_addr_generator/gpu.rs` plus vendored kernels under `src/wgpu_sig_ops/`. `vanity_addr.rs` keeps shared GPU engine/tuning caches and decides fallback behavior.

## Key repository-specific conventions

- Feature gating is a core design choice:
  - Bitcoin is always present.
  - Ethereum/Solana modules and APIs are conditionally compiled (`#[cfg(feature = "...")]`).
  - GPU backend is conditionally compiled via `gpu`.
- Backend semantics are intentional:
  - CLI/backend parser accepts `both` as alias for `hybrid`.
  - Regex matching is CPU-only. Explicit `gpu` + regex returns `GpuRegexUnsupported`; `hybrid`/`auto` regex paths use CPU.
  - In `auto`, if `gpu_batch_size <= 8_192`, search is forced to CPU.
- Pattern normalization is chain-specific and should be preserved when extending behavior:
  - Bitcoin prefix mode prepends `1` internally.
  - Bitcoin regex beginning with `^` is rewritten to `^1...` when needed.
  - Ethereum case-sensitive matching is unsupported; Ethereum regex is normalized to lowercase and strips leading `^0x`.
- Input file parsing (`src/file.rs`) treats each non-comment line as: `<pattern> [flags...]`. Without `--force-flags`, per-line flags override CLI defaults for that pattern.
- `VanitySearchOptions` is the stable extension point for new search knobs; prefer adding new execution controls there rather than expanding positional APIs.
