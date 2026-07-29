<!-- PROJECT LOGO -->
<br />
<div align="center">
  <p>
    <img src="images/bitcoin.svg" alt="Bitcoin" height="54">
    &nbsp;&nbsp;
    <img src="images/ethereum.png" alt="Ethereum" height="54">
    &nbsp;&nbsp;
    <img src="images/solana.png" alt="Solana" height="54">
  </p>

  <h1 align="center">btc-vanity</h1>

  <p align="center">
    Blazingly fast local vanity-address search for Bitcoin, Ethereum, and Solana.<br />
    Multithreaded CPU search, GPU search, or both (Hybrid mode).
  </p>

  <p align="center">
    <a href="https://crates.io/crates/btc-vanity"><img src="https://img.shields.io/crates/v/btc-vanity.svg" alt="crates.io release"></a>
    <a href="https://crates.io/crates/btc-vanity"><img src="https://img.shields.io/crates/d/btc-vanity.svg" alt="crates.io downloads"></a>
    <a href="https://docs.rs/btc-vanity/latest/btc_vanity/"><img src="https://docs.rs/btc-vanity/badge.svg" alt="docs.rs"></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.89%2B-black.svg" alt="Rust 1.89+"></a>
    <a href="https://github.com/Emivvvvv/btc-vanity/actions/workflows/rust.yml"><img src="https://github.com/Emivvvvv/btc-vanity/actions/workflows/rust.yml/badge.svg" alt="CI"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green.svg" alt="Apache-2.0"></a>
  </p>

  <p align="center">
    <a href="https://emivvvvv.github.io/btc-vanity/"><strong>Read the user manual »</strong></a>
    <br />
    <a href="https://docs.rs/btc-vanity/latest/btc_vanity/">API reference</a>
    ·
    <a href="https://github.com/Emivvvvv/btc-vanity/issues/new?labels=bug">Report a bug</a>
    ·
    <a href="https://github.com/Emivvvvv/btc-vanity/issues/new?labels=enhancement">Request a feature</a>
  </p>
</div>

---

A blazingly fast vanity address generator written in Rust. Search Bitcoin
(always included), Ethereum, and Solana (via optional features) locally for
custom prefixes, suffixes, substrings, or regex patterns using the CLI or Rust library.

## Why btc-vanity?

- **Blazingly fast performance.** Multithreaded CPU search and experimental GPU acceleration deliver high-throughput address generation.
- **One search interface, three chains.** Search Bitcoin, Ethereum, and Solana
  addresses from the CLI or the Rust library.
- **Local key generation.** Candidate key material stays within the local
  process and, when experimental GPU acceleration is enabled, the local
  graphics stack.
- **Backends for different workloads.** Choose predictable CPU execution,
  adaptive Auto selection, concurrent Hybrid search, or the experimental GPU
  backend explicitly.
- **Matching without workflow changes.** Prefix, suffix, and anywhere searches
  share the same options; regex routes to the CPU.
- **Resource-aware acceleration.** Best-effort GPU duty-cycle control trades
  throughput for a more responsive desktop.

## Quick start

Install Bitcoin CPU support:

```bash
cargo install btc-vanity
```

Install every chain plus experimental GPU support:

```bash
cargo install btc-vanity --features all
```

Bitcoin and prefix matching are the defaults:

```bash
btc-vanity emiv
```

Select another chain or match mode:

```bash
btc-vanity --eth --suffix dead
btc-vanity --sol --anywhere --case-sensitive Sun
btc-vanity --backend cpu --regex '^1E.*T$'
```

Use Auto when the program should select a backend from the pattern and
available hardware:

```bash
btc-vanity --backend auto --anywhere test
```

Run `btc-vanity --help` for the complete option list. Ethereum, Solana, and
experimental GPU commands require the corresponding Cargo features.

## Capability matrix

| Capability | Bitcoin | Ethereum | Solana |
| --- | --- | --- | --- |
| Address form searched | Mainnet P2PKH, Base58 | Hexadecimal | Base58 |
| Prefix | Yes | Yes | Yes |
| Suffix | Yes | Yes | Yes |
| Anywhere | Yes | Yes | Yes |
| Regex | Yes, CPU | Yes, CPU | Yes, CPU |
| Case-sensitive exact matching | Yes | No | Yes |
| CPU | Built in | `ethereum` feature | `solana` feature |
| Experimental GPU exact matching | `gpu` feature | `ethereum` + `gpu` | `solana` + `gpu` |

The `all` feature enables `ethereum`, `solana`, and the experimental `gpu`
feature. Bitcoin CPU search needs no optional feature.

## Execution backends

| Backend | Behavior | Best fit |
| --- | --- | --- |
| `cpu` | Multithreaded CPU search for every match mode | Regex, short patterns, compatibility, or CPU-only operation |
| `auto` | Selects CPU, Hybrid, or experimental GPU for exact matching; uses CPU for regex and falls back to CPU when acceleration is unavailable | Let the search shape and available adapter drive selection |
| `hybrid` | Runs CPU and experimental GPU workers together for exact matching; uses CPU for regex | Keep CPU search active while using a compatible adapter |
| `gpu` | Uses only the experimental GPU path and rejects regex or an unavailable GPU build/adapter | Explicit accelerator-only exact matching |

The CLI defaults to Hybrid; `VanitySearchOptions::default()` uses Auto for
library calls. Without the `gpu` feature, Hybrid runs on CPU.

Hybrid defaults to 70% GPU usage. Explicit GPU defaults to 100%:

```bash
btc-vanity --backend hybrid --gpu-usage-limit 50 emiv
btc-vanity --backend gpu --gpu-usage-limit 90 emiv
```

`--gpu-usage-limit` accepts values from 1 through 100. It is a best-effort
dispatch duty-cycle limit, not a hardware power cap. Drivers, operating-system
scheduling, and competing workloads still determine responsiveness. Lower the
value when the display or another GPU workload needs more headroom.

`--gpu-batch-size` is an advanced override for experimental GPU and Hybrid
search. Larger work submissions can favor throughput while increasing the time
before other graphics work is scheduled.

## Library use

```toml
[dependencies]
btc-vanity = { version = "3.0.0", features = ["all"] }
```

```rust
use btc_vanity::{
    BitcoinKeyPair, VanityAddr, VanityBackend, VanityMode, VanitySearchOptions,
};

fn main() -> Result<(), btc_vanity::error::VanityError> {
    let wallet = VanityAddr::generate_with_options::<BitcoinKeyPair>(
        "emiv",
        VanitySearchOptions {
            threads: 4,
            case_sensitive: false,
            vanity_mode: VanityMode::Prefix,
            backend: VanityBackend::Hybrid,
            gpu_usage_limit: Some(70),
            ..VanitySearchOptions::default()
        },
    )?;

    println!("address: {}", wallet.get_comp_address());
    Ok(())
}
```

This example prints only the public address. Generated wallet objects also
contain private key material; keep them out of logs and diagnostics.

## Measured performance

The following fixed-work Bitcoin P2PKH candidate loops were measured on an
Apple M1 Pro with an 8-core CPU, 14-core integrated GPU, 16 GiB of unified
memory, and Metal 3. Each path performed compressed public-key derivation,
address generation, and a case-sensitive prefix comparison.

| Tool and path | Median | Throughput |
| --- | ---: | ---: |
| btc-vanity CPU, one worker | 304.019 ms / 16,384 candidates | **53,891 candidates/s** |
| [vgen 0.3.0](https://github.com/oritwoen/vgen/tree/a405fcb69032d328318cfacbda9d7fea9d4dcacc) CPU, one worker | 21.432 µs / candidate | 46,659 candidates/s |
| btc-vanity Metal, default batch and ring depth | 5.001 s / 524,288 candidates | **104,837 candidates/s** |

For this single-worker Bitcoin loop, btc-vanity CPU was approximately **15.5%
faster than vgen**, while Metal delivered approximately **1.95× btc-vanity's
one-worker CPU throughput**. Results are hardware-specific and do not compare
full multicore CPU execution. See the [methodology and
limitations](https://emivvvvv.github.io/btc-vanity/performance.html).

## Security and limitations

btc-vanity creates spend-authorizing private keys. Treat the process, terminal,
graphics stack, output files, backups, and wallet import path as one security
boundary.

- Run a reviewed build on a machine you control. Experimental GPU search sends
  seed and candidate state to the local graphics device and driver; use CPU if
  that stack is outside your trust boundary.
- The CLI prints wallet details unless `--output-file` is used. Terminal
  scrollback, session recording, existing file permissions, and backups can
  retain private keys.
- Newly created output files request owner-only permissions on Unix. Existing,
  shared, network, and removable filesystems may behave differently; verify
  permissions yourself.
- Independently derive and verify the returned address before funding it. Never
  place a real private key in a website, issue, test fixture, log, or chat.
- A vanity pattern changes an address's appearance, not the security of its
  private key. Search duration is not guaranteed.
- Regex is CPU-only. Explicit experimental GPU regex requests fail; Auto and
  Hybrid route regex to CPU.
- Ethereum exact matching is case-insensitive. Bitcoin output is mainnet P2PKH.
- Experimental GPU support requires the `gpu` feature, a compatible wgpu
  adapter, and a working platform graphics stack.

Read the [security guide](docs/src/security.md) before using a generated wallet
with funds.

## Explore

- [User manual](https://emivvvvv.github.io/btc-vanity/) - guided installation
  and usage
- [CLI reference](docs/src/cli.md) - chains, match modes, files, and backend
  controls
- [Experimental GPU and Hybrid search](docs/src/gpu-search.md) - adapter
  behavior, resource limits, and tuning
- [Library guide](docs/src/library.md) - typed search APIs and chain-specific
  examples
- [API reference](https://docs.rs/btc-vanity/latest/btc_vanity/) - public Rust
  interface
- [Security guide](docs/src/security.md) - key handling and trust boundaries
- [Contributing](CONTRIBUTING.md) - development workflow and verification
  requirements

## License

btc-vanity is licensed under the [Apache License 2.0](LICENSE). Adapted portions
of the experimental GPU implementation retain the
[wgpu-sigops MIT license](THIRD_PARTY_LICENSES/wgpu-sigops/LICENSE-MIT).

## Disclaimer

**USE WITH CAUTION AND UNDERSTANDING**

btc-vanity is a tool designed to assist users in generating customized vanity addresses (Bitcoin, Ethereum, Solana) using the Rust programming language. While btc-vanity aims to provide a secure and efficient method for generating vanity addresses, it is essential to exercise caution and follow best security practices.

1. **Security Awareness**: Generating and using vanity addresses involves the creation of private keys and public addresses. Private keys grant control over the associated funds. It is crucial to understand the risks involved in managing private keys and to never share them with anyone. Keep your private keys stored securely and never expose them to potential threats.

2. **Risk of Loss**: Improper use of btc-vanity, mishandling of private keys, or failure to follow security guidelines may result in the loss of funds. Always double-check the addresses generated and verify their accuracy before using them for transactions.

3. **Verification**: Before utilizing any vanity address generated by btc-vanity, thoroughly verify the integrity of the software and the generated addresses. Only use versions of btc-vanity obtained from reputable sources, such as the official crates.io page.

4. **Backup and Recovery**: Maintain proper backups of your private keys and any relevant data. In the event of device failure, loss, or corruption, having secure backups will help prevent irreversible loss of funds.

5. **Use at Your Own Risk**: The btc-vanity software is provided "as is," without any warranties or guarantees. The author(s) and contributors of btc-vanity shall not be held responsible for any direct or indirect damages, losses, or liabilities resulting from the use or misuse of this software.

6. **Educational Purposes**: btc-vanity is intended for educational and personal use. It is your responsibility to ensure compliance with any legal, regulatory, or tax requirements in your jurisdiction related to cryptocurrency usage.

By using btc-vanity, you acknowledge and accept the risks associated with generating vanity addresses and handling private keys. It is your responsibility to exercise diligence, follow security best practices, and be aware of potential risks. If you are unsure about any aspect of using btc-vanity, seek guidance from experienced cryptocurrency users or professionals before proceeding.

Remember, the security of your crypto holdings is paramount. Always prioritize the safety and security of your assets.
