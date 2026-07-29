# Rust library

Add only the features your application needs:

```toml
[dependencies]
btc-vanity = { version = "3.0.0", features = ["ethereum", "gpu"] }
```

Bitcoin is always available. `EthereumKeyPair` and `SolanaKeyPair` are exported
only with their respective features.

## Configured exact search

```rust,ignore
use btc_vanity::{
    BitcoinKeyPair, VanityAddr, VanityBackend, VanityMode, VanitySearchOptions,
};

let result = VanityAddr::generate_with_options::<BitcoinKeyPair>(
    "abc",
    VanitySearchOptions {
        threads: 4,
        case_sensitive: false,
        fast_mode: true,
        vanity_mode: VanityMode::Prefix,
        backend: VanityBackend::Cpu,
        gpu_batch_size: None,
        gpu_usage_limit: None,
    },
)?;

assert!(
    result
        .get_comp_address()
        .to_ascii_lowercase()
        .starts_with("1abc")
);

# Ok::<(), btc_vanity::error::VanityError>(())
```

`VanitySearchOptions::default()` uses available CPU parallelism, insensitive
exact matching, fast mode, prefix mode, Auto backend, and no GPU overrides.
This Auto default differs from the CLI's Hybrid default.

The older positional `VanityAddr::generate` method remains available. It uses
Auto and accepts `pattern`, `threads`, `case_sensitive`, `fast_mode`, and
`VanityMode`.

## Regex search

```rust,ignore
use btc_vanity::{BitcoinKeyPair, VanityAddr};

let result =
    VanityAddr::generate_regex::<BitcoinKeyPair>("^1A.*Z$", 4)?;
assert!(result.get_comp_address().starts_with("1A"));

# Ok::<(), btc_vanity::error::VanityError>(())
```

Use `generate_regex_with_options` to select a backend explicitly. CPU, Auto,
and Hybrid execute regex on CPU. Explicit GPU returns
`VanityError::GpuRegexUnsupported`.

## Result types

All chain keypair types implement `KeyPairGenerator`, including
`get_address()` and `get_address_bytes()`.

- `BitcoinKeyPair` exposes the private key, compressed public key, compressed
  address, WIF private key, and compressed public-key text.
- `EthereumKeyPair` exposes secp256k1 key references and hexadecimal private
  key, public key, and address helpers.
- `SolanaKeyPair` exposes the underlying keypair plus Base58 private-key and
  public-key helpers.

Treat every private-key accessor as a secret-handling boundary.

## Errors

Configured methods return `Result<T, VanityError>`. Callers should distinguish:

- invalid Base58, hexadecimal, or regex input;
- fast-mode and absolute-length rejections;
- zero threads or an invalid GPU usage limit;
- missing optional chain features;
- unavailable, unsupported, or uninitialized GPU paths;
- GPU regex requests; and
- a GPU result that fails reconstruction.

Auto and Hybrid define CPU fallback. Do not add an automatic fallback around
explicit GPU unless that is the application's intended policy.
