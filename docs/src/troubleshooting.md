# Troubleshooting

Most failures fall into input validation, missing features, backend
availability, or output handling.

## Input is not Base58 encoded

Bitcoin and Solana patterns cannot contain `0`, `O`, `I`, or lowercase `l`.
Use only:

```text
123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz
```

Ethereum patterns are hexadecimal instead.

## Input is not Base16 encoded

Enable the `ethereum` feature, select `--eth`, and use only `0-9`, `a-f`, or
`A-F`. Omit the display prefix `0x` from exact patterns.

## Fast mode enabled, input is too long

Fast mode permits at most 5 characters for Bitcoin and Solana or 16 for
Ethereum. `--disable-fast` permits longer input up to the absolute limits of 25
and 40 respectively.

This flag removes a guard; it does not make the longer request practical.
Estimate or benchmark the cost before continuing.

## Invalid Regex or regex alphabet error

The pattern must both compile and pass chain-specific validation. Only
alphanumeric literals from the chain's address alphabet and the documented
metacharacters are accepted. Regex search is case-sensitive and CPU-only.

Use an exact mode when you only need prefix, suffix, or substring matching.

## Ethereum or Solana support is not enabled

Reinstall or rebuild with the matching Cargo feature:

```bash
cargo install btc-vanity --features ethereum
cargo install btc-vanity --features solana
```

Combine a chain feature with `gpu` when both are required.

## GPU backend is unavailable

There are three common cases:

- the binary was built without `gpu`;
- wgpu could not obtain a compatible adapter; or
- the driver could not create the required device.

Use `--backend cpu` to continue without GPU. Auto and Hybrid can fall back to
CPU. If GPU is required, update the graphics driver and confirm that the
platform provides a working Metal, Vulkan, or DirectX 12 path.

## Regex is not supported by GPU

Choose CPU, Auto, or Hybrid. Explicit GPU deliberately returns an error for
regex rather than silently changing the requested backend.

## The display stutters

Lower `--gpu-usage-limit`, reduce `--gpu-batch-size`, or use CPU. The limit is
best-effort, and driver scheduling can still produce visible stalls. Large
batches also delay Hybrid cancellation.

## Search appears to take forever

There is no deterministic completion deadline. Confirm that:

- the pattern uses the selected chain's alphabet;
- the case and match mode are what you intended;
- Bitcoin prefix input omits the fixed leading `1`;
- the release binary is being used;
- the pattern length is realistic for the measured candidate rate; and
- power saving or thermal throttling has not reduced performance.

Stop and shorten the pattern if the expected resource use is not acceptable.

## Output file is missing or too permissive

The parent directory must exist and be writable. Output is appended. On Unix,
mode `0600` is requested only when a file is created; an existing file retains
its mode. Check permissions directly and avoid destinations whose filesystem
cannot enforce the intended access.

Any item-generation or write failure makes the final process status nonzero. In
batch mode, inspect standard error for the specific row failure.
