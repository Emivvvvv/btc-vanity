# GPU engine

The experimental GPU engine is a persistent wgpu compute pipeline for exact
matching. It supports Metal, Vulkan, and DirectX 12 through wgpu rather than
maintaining separate chain implementations for each graphics API.

## Shader construction

Curve arithmetic, finite-field operations, hash functions, Base58, and
chain-specific search entry points are embedded as WGSL source. At
initialization, a template renderer inserts modulus, Montgomery-arithmetic,
limb, generator, and window-table constants into complete Bitcoin, Ethereum,
and Solana shaders.

The engine creates all three chain pipelines when the shared GPU engine is
first initialized. This cold setup includes adapter and device creation,
precomputed secp256k1 and Ed25519 tables, shader rendering, and compute-pipeline
compilation. The engine is cached for later searches in the same process.

## Buffers and transfers

Each chain pipeline keeps a seed buffer, pattern buffer, compute pipeline, and
several dispatch slots. Precomputed curve tables remain in storage buffers.
Each slot has:

- a candidate counter;
- compact dispatch parameters;
- an atomic result buffer;
- a small status readback buffer; and
- a full result readback buffer.

The host writes seed and pattern data once per search. Per dispatch it updates
parameters and resets the winner sentinel. After compute, it copies back only
the first result word to learn whether a winner exists. The larger result is
transferred only after that status indicates a match.

This avoids transferring every candidate public key or address across the
device boundary.

## On-device work

For each candidate, the chain shader performs:

- scalar or seed progression;
- secp256k1 or Ed25519 public-key derivation using precomputed tables;
- Bitcoin SHA-256, RIPEMD-160, checksum, and Base58Check; Ethereum Keccak-256
  and hexadecimal encoding; or Solana Ed25519 public-key encoding in Base58;
- prefix, suffix, or anywhere comparison; and
- an atomic attempt to claim the result buffer.

Atomic claiming allows only one invocation to publish the winner fields even
when many GPU invocations match concurrently.

## Dispatch slots and throttling

Normal tuning uses a 256-thread workgroup, several candidates per invocation,
a 262,144-candidate batch, and two active slots. Slots let the host submit new
work while checking earlier status readbacks.

Short patterns favor a smaller batch and one slot because a result is likely
before peak throughput matters. A usage limit below 100% caps the batch at
4,096, uses one slot, and inserts idle time after active work.

## Host reconstruction

The winner contains private-key bytes and encoded address data. The host rebuilds
the chain keypair on CPU and compares its derived address with the shader
result. A disagreement fails the request. Hybrid shares a stop flag so a CPU
winner prevents additional GPU submission as soon as the host can observe it;
already-running dispatch work still has normal batch latency.
