# Command-line reference

```text
btc-vanity [OPTIONS] [PATTERN]
```

Provide either `PATTERN` or `--input-file FILE`. Bitcoin, prefix matching, and
Hybrid are the CLI defaults.

## Chain selection

| Flag | Meaning |
| --- | --- |
| `--btc` | Bitcoin P2PKH; default |
| `--eth` | Ethereum; requires `ethereum` |
| `--sol` | Solana; requires `solana` |

Chain flags conflict. A binary without the requested optional chain feature
reports the missing feature when generation begins.

## Input and output

| Flag | Meaning |
| --- | --- |
| `-i`, `--input-file FILE` | Read one pattern and optional flags per line |
| `-o`, `--output-file FILE` | Append wallet details to a file |
| `-f`, `--force-flags` | Ignore row flags and apply CLI flags to every row |

See [Batch input and output](batch.md) for syntax, precedence, and secret-file
handling.

## Match mode

| Flag | Meaning |
| --- | --- |
| `-p`, `--prefix` | Match the address start; default |
| `-s`, `--suffix` | Match the address end |
| `-a`, `--anywhere` | Match at any position |
| `-r`, `--regex` | Use a CPU regular expression |

These flags conflict. `--regex` is a switch; the positional `PATTERN` contains
the expression:

```bash
btc-vanity --backend cpu --regex '^1A.*Z$'
```

## Performance

| Flag | Meaning |
| --- | --- |
| `-t`, `--threads N` | CPU worker count; defaults to available parallelism |
| `-b`, `--backend NAME` | `auto`, `cpu`, `gpu`, `hybrid`, or alias `both` |
| `--gpu-batch-size N` | Requested GPU candidates per batch |
| `--gpu-usage-limit PERCENT` | Best-effort GPU dispatch duty cycle, `1..=100` |

Thread count must be at least 1. Hybrid's default limit is 70%; explicit GPU's
is 100%. GPU flags have no effect on a CPU-only search.

## Matching behavior

| Flag | Meaning |
| --- | --- |
| `-c`, `--case-sensitive` | Distinguish ASCII case for Bitcoin or Solana exact matching |
| `-d`, `--disable-fast` | Remove the short-pattern guard; absolute limits remain |

`--case-sensitive` conflicts with `--eth`. Exact Base58 patterns are limited to
5 characters in fast mode and 25 when fast mode is disabled. Ethereum limits
are 16 and 40 hexadecimal characters.

## Exit behavior

Argument errors, input-file errors, generation errors, and output-write errors
produce a nonzero exit status. In batch mode, processing continues after an
individual item fails and the final status is nonzero if any item failed.

Use the installed binary's help for the exact options in that version:

```bash
btc-vanity --help
```
