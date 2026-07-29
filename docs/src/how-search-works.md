# How a search works

Every search has the same high-level data flow:

```text
pattern and options
        |
        v
chain validation and adjustment
        |
        v
candidate private key -> public key -> address text
        |
        v
prefix / suffix / anywhere / regex comparison
        |
        v
first valid matching keypair
```

## Validation comes first

The selected chain validates the alphabet, length, case policy, and regex
syntax. Bitcoin then inserts its fixed leading `1` into prefix comparisons.
Ethereum normalizes regex text to lowercase and works on the unprefixed internal
address. Backend selection happens after this chain-specific preparation.

An invalid request fails before workers begin.

## Candidate generation

CPU workers generate independent random key material through thread-local random
number generators. Each candidate follows the normal chain pipeline; no address
characters are patched after derivation.

The GPU path begins with a random 32-byte seed and enumerates candidate scalars
or seeds in compute work. Precomputed curve tables accelerate public-key
derivation, while the shader performs the chain's hash and encoding operations.

## First-result coordination

CPU threads share an atomic stop flag. The first matching worker claims it,
sends its owned keypair over a channel, and causes other workers to stop.

GPU work uses an atomic result claim inside each shader result buffer. The host
first reads a small winner status. Only when a winner exists does it transfer
the complete result. Hybrid adds a shared stop flag between its CPU and GPU
workers and returns whichever valid path finishes first.

## Defense after a GPU match

The GPU reports candidate private bytes and address text. The host reconstructs
the chain keypair from those private bytes using the CPU implementation and
requires the reconstructed address to equal the GPU-reported address. A
mismatch is returned as an invalid-GPU-result error.

This check detects a bad returned pair; it is not a substitute for independent
wallet verification or secure key handling.
