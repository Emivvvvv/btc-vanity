# Batch input and output

Use an input file to run several searches in sequence:

```bash
btc-vanity --input-file patterns.txt
```

Each nonempty, noncomment line begins with one pattern. Remaining
whitespace-separated tokens are per-line flags:

```text
# patterns.txt
abc --btc --prefix --backend cpu
dead --eth --suffix --backend gpu --gpu-usage-limit 60
Sun --sol --anywhere --case-sensitive --threads 4
```

There is no shell-style quoting in this format. The pattern is the first token,
so it cannot contain whitespace. Empty lines and lines whose first
non-whitespace character is `#` are ignored.

## Supported line flags

Batch rows recognize the chain, match-mode, case, fast-mode, output, threads,
backend, GPU batch size, and GPU usage-limit flags described in the
[command-line reference](cli.md). Unrecognized or malformed row options are
ignored rather than reported as command-line parse errors, so review batch
files carefully.

## Flag precedence

Without `--force-flags`, row settings take precedence:

- a row's chain, mode, backend, output file, GPU settings, or positive thread
  count overrides the command line;
- omitted optional row values fall back to the corresponding command-line
  value; and
- case sensitivity and fast-mode disabling are row-local booleans. Put
  `--case-sensitive` or `--disable-fast` on every row that needs it.

With `--force-flags`, all row flags are ignored and the command-line
configuration applies to every pattern:

```bash
btc-vanity --input-file patterns.txt \
  --force-flags --btc --prefix --backend cpu --threads 4
```

The first token on each row remains the pattern.

## Output destinations

Without `--output-file`, each result is printed to the terminal. With an output
file, wallet details are appended:

```bash
btc-vanity --output-file generated-wallets.txt abc
```

For a batch, a row-level output path overrides the global output path unless
`--force-flags` is active.

The output includes private key material:

- Bitcoin: WIF private key, compressed public key, and P2PKH address
- Ethereum: hexadecimal private key, uncompressed public key, and address
- Solana: hexadecimal keypair bytes and address

On Unix, a newly created file requests mode `0600`. Opening an existing file
does not repair its permissions, and some network or removable filesystems do
not enforce Unix modes. Verify the destination before and after a run. Output
is appended, never replaced.

If any row fails to generate or write, the process continues through the batch
but exits with a nonzero status at the end.
