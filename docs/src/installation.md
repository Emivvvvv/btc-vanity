# Installation

btc-vanity requires Rust 1.89 or newer. The default build includes Bitcoin CPU
search and no optional features.

## Install the command-line program

Install the smallest feature set that covers your use:

```bash
# Bitcoin on CPU
cargo install btc-vanity

# Bitcoin with experimental GPU support
cargo install btc-vanity --features gpu

# Add one optional chain
cargo install btc-vanity --features ethereum
cargo install btc-vanity --features solana

# Bitcoin, both optional chains, and GPU
cargo install btc-vanity --features all
```

The Cargo features are:

| Feature | Adds |
| --- | --- |
| `ethereum` | Ethereum key and address support |
| `solana` | Solana key and address support |
| `gpu` | Experimental GPU and Hybrid acceleration through wgpu |
| `all` | `ethereum`, `solana`, and `gpu` |

Features are additive. For example, Ethereum with GPU support uses
`--features ethereum,gpu`.

## Build a checkout

```bash
git clone https://github.com/Emivvvvv/btc-vanity.git
cd btc-vanity
cargo build --release --features all
./target/release/btc-vanity --help
```

Use the release binary for meaningful performance measurements. Debug builds
favor diagnostics over search throughput.

## GPU requirements

The `gpu` feature uses wgpu, which reaches native Metal, Vulkan, or DirectX 12
backends depending on the platform and driver. Building the feature does not
guarantee that a usable adapter will be available at runtime.

If explicit GPU initialization fails, update the graphics driver or use
`--backend cpu`. Auto and Hybrid can continue on CPU when GPU work is not
available. See [Troubleshooting](troubleshooting.md) for specific errors.
