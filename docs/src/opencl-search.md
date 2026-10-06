# OpenCL Bitcoin search

The `opencl` feature adds a dedicated Bitcoin prefix search for OpenCL GPUs.
It is separate from the wgpu backend and trades generality for throughput:
Bitcoin only, prefix matching only, but on the order of a billion keys per
second on a modern integrated or discrete GPU.

```bash
cargo install btc-vanity --features opencl
btc-vanity --opencl -c -o wallet.txt 1Emiv
```

A missing leading `1` is added, so `--opencl emiv` searches for `1emiv`.
Case-insensitive searches (the default) cover every base58-valid casing of the
pattern in a single pass at no extra cost per key.

## How it is fast

Each OpenCL work-item owns a random starting key `k` and its point `k*G`.

- **Batched affine additions.** One dispatch walks the `2*group+1`
  consecutive keys around the centre. The points come from one affine
  addition each against a precomputed table of `i*G`. All the additions share
  a single field inversion (Montgomery's batch trick). No work-item ever
  performs a scalar multiplication on the GPU.
- **Six keys per point.** The secp256k1 endomorphism maps `(x, y)` to
  `(beta*x, y)` for the key `lambda*k`, and negation flips the parity of `y`
  for `-k`. Each computed point therefore yields six distinct compressed public
  keys, and the field arithmetic is amortised away. The SHA-256 and
  RIPEMD-160 work dominates.
- **No base58 on the GPU.** The host converts the requested prefix (and every
  case variant) into sorted ranges over the top 64 bits of HASH160. A
  candidate matches when its HASH160 falls inside a range, which a binary
  search decides in a few comparisons.

## Verification

The GPU reports only the work-item, offset, endomorphism power and sign of a
hit. The CPU reconstructs the private key, derives the address with the
`bitcoin` crate, and checks the pattern before anything is returned. A range
boundary can produce a rare hit that fails this check; it is discarded.

`tests/opencl_backend_tests.rs` runs short patterns that produce hundreds of
hits and asserts that every hit verifies and that the hit rate matches the
expected probability.

## Tuning

| Flag | Default | Meaning |
| --- | --- | --- |
| `--ocl-items N` | 65536 | Concurrent work-items, each with its own walking key |
| `--ocl-group N` | 256 | Points on each side of a work-item's centre per dispatch |

Each dispatch checks `items * (2*group+1) * 6` keys. Keep a dispatch well
under a second so the desktop stays responsive and the OS GPU watchdog never
fires. The scratch buffer needs `(group+1) * 32 * items` bytes.

`cargo bench --features opencl --bench opencl_btc` prints throughput for a
few configurations. On an AMD Radeon 8060S (gfx1151, Ryzen AI MAX+ 395):

| items | group | Mkeys/s |
| ---: | ---: | ---: |
| 16384 | 256 | 1446 |
| 32768 | 256 | 1541 |
| 65536 | 256 | 1583 |
| 65536 | 128 | 1521 |

## Limits

- Bitcoin mainnet P2PKH with compressed keys only.
- Prefix mode only, and the prefix may not start with `11`.
- Needs an OpenCL runtime with a GPU device. The library loads it at run time,
  so the build itself does not need OpenCL headers or import libraries.
- As with the wgpu backend, key material passes through the local GPU driver.
  See [Security](security.md).
