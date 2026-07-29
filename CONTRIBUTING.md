# Contributing

Bug reports and focused pull requests are welcome. Describe the user-visible
problem, the intended behavior, and any correctness, performance,
compatibility, or secret-handling trade-off.

## Set up a development checkout

The project requires Rust 1.89 or newer. Build the full feature set before
changing behavior shared by chains or backends:

```bash
cargo build --locked --all-features
```

Keep generated wallet credentials, terminal captures containing keys, benchmark
artifacts containing secrets, and local planning files out of version control.
Use disposable test material only.

## Required verification

Run the checks relevant to the change, then run the complete set before
submission:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --locked --no-default-features
cargo test --locked --doc --all-features
cargo package --locked --features all
mdbook test docs
mdbook build docs
```

Tests that execute `target/debug/btc-vanity` expect the binary to have been
built with the feature set under test.

## GPU changes

GPU work must preserve CPU/GPU equivalence for known inputs, valid winner
reconstruction, atomic result claiming, and fallback semantics. Run the
adapter-backed integration tests on a machine with the target GPU and driver:

```bash
cargo test --locked --all-features --test gpu_backend_tests -- --nocapture
```

Report the operating system, GPU model, driver, and graphics API. A skipped
adapter test is not evidence that the GPU path works on that platform. Changes
to WGSL or buffer layouts should include focused tests for shader construction,
host/device agreement, and concurrent winner publication.

## Benchmarks

Use release-style Criterion measurements:

```bash
cargo bench --locked --features all --bench gpu_end_to_end
```

The benchmark covers cold GPU initialization and fixed-work CPU/GPU chain
pipelines. Full-throughput GPU runs can make an interactive desktop
temporarily unresponsive; run them on an otherwise idle machine.

Any published performance result must include:

- the exact revision and command;
- Rust version and locked dependencies;
- CPU, GPU, memory, operating system, driver, and graphics API;
- chain, mode, case policy, pattern, worker count, GPU batch size, and usage
  limit;
- cold versus warm state, sample count, and summary statistic;
- result validation and any failed or skipped samples; and
- raw machine-readable output sufficient to reproduce the summary.

Do not publish competitor rankings, universal speedup claims, or best-sample
figures. Compare equivalent full pipelines and explain uncertainty.

## Documentation and release review

The mdBook under `docs/` is the user manual. Keep development and release
commands in this file rather than adding user-facing development chapters.
Examples must never contain real private keys.

Before release, perform the complete verification on a clean checkout, inspect
the packaged crate contents, review dependency advisories and third-party
licenses, and test every advertised GPU platform on real hardware.
