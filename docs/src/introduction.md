# Introduction

btc-vanity is a blazingly fast local vanity-address search tool for Bitcoin,
Ethereum, and Solana addresses whose text matches a pattern you choose. It
generates candidates locally, derives each candidate's address, and stops when
one matches. The result is a normal keypair: the vanity text changes how the
address looks, not how the network treats it.

The program offers four execution backends:

| Backend | What it does |
| --- | --- |
| CPU | Runs worker threads on the processor |
| GPU | Runs an exact-pattern compute pipeline on a graphics adapter |
| Hybrid | Races CPU and GPU work and returns the first valid result |
| Auto | Chooses a backend from the pattern and available build support |

GPU support is experimental. Adapter behavior and responsiveness vary with the
operating system, graphics API, driver, and workload. CPU search is the most
portable option.

## What the program supports

- Bitcoin mainnet P2PKH addresses with compressed public keys
- Lowercase Ethereum addresses, displayed with a `0x` prefix by the CLI
- Solana public-key addresses
- Prefix, suffix, and substring matching
- CPU regular expressions
- Single searches and line-oriented batch files
- A command-line program and a generic Rust API

## Set expectations before a long search

A search samples independent keys until one address matches. It cannot edit an
existing address, preserve an existing private key, or guarantee a completion
time. Every extra constrained character makes the target rarer, so difficulty
usually grows exponentially.

Start with two or three characters, measure your actual machine, and extend the
pattern only after the expected wait and energy cost are acceptable. A fast
result is possible even for a difficult pattern, but so is a run much longer
than the average.

## A safe reading path

1. Learn [what a vanity address is](vanity-address.md).
2. [Install](installation.md) only the chain and backend features you need.
3. Complete [your first search](first-search.md) with a short test pattern.
4. Read [Security](security.md) before receiving funds at a generated address.
5. Use [Choosing a backend](backends.md) before committing to a longer search.
