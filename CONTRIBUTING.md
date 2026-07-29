# Contributing

Contributions to `btc-vanity` are welcome! Please follow these development workflows and documentation requirements when contributing.

## Development Workflows

### Setup
Rust 1.89 or newer is required.

```bash
cargo build --locked --all-features
```

### Verification & Testing Workflow
Run the full verification suite before submitting pull requests:

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

### Benchmarking & Performance Workflows
If an update touches the search engine or might affect performance:

1. **Benchmark difference results**: If an existing benchmark covers your changes, run it before and after your edits and include the **benchmark comparison/difference results** in your pull request.
2. **Benchmark creation**: If no benchmark exists for the modified engine path or search feature, you are required to **create a new benchmark** under `benches/` (covering CPU, GPU, or end-to-end performance as applicable) and include its results.

```bash
cargo test --locked --all-features --test gpu_backend_tests -- --nocapture
cargo bench --locked --features all
```

## Documentation & Book Updates

If your pull request introduces new features, changes CLI flags, or modifies library APIs:

1. **User Manual (mdBook)**: Update the appropriate documentation pages under `docs/src/` (e.g. `cli.md`, `chains.md`, `library.md`, `backends.md`).
2. **README.md**: Update feature lists, capability matrices, or code examples if affected.
3. **Validation**: Run `mdbook test docs` and `mdbook build docs` to ensure all book tests pass and the book builds cleanly.
