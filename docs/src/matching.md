# Matching

btc-vanity has three exact-pattern modes and one regular-expression mode.

| Mode | CLI flag | Match location | Backend support |
| --- | --- | --- | --- |
| Prefix | `--prefix`, `-p` | Start of address | CPU, GPU, Hybrid, Auto |
| Suffix | `--suffix`, `-s` | End of address | CPU, GPU, Hybrid, Auto |
| Anywhere | `--anywhere`, `-a` | Any position | CPU, GPU, Hybrid, Auto |
| Regex | `--regex`, `-r` | Rust regular expression | CPU; Auto/Hybrid route to CPU |

Prefix is the default. Match-mode flags conflict, so select at most one.

## Chain adjustments

Bitcoin prefix search prepends the address's fixed `1` to the pattern. A pattern
of `abc` therefore tests for `1abc`. Bitcoin regex patterns beginning with `^`
are similarly adjusted to include `1` unless they already begin with `^1`.

Ethereum exact matching uses the internal 40-character hexadecimal address.
Do not include `0x` in an exact pattern.

Solana performs no prefix adjustment.

## Case rules

Exact matching is ASCII case-insensitive by default. Add `--case-sensitive` for
Bitcoin or Solana when uppercase and lowercase must be distinct.

Ethereum addresses are generated as lowercase hexadecimal. The CLI prevents
combining `--eth` and `--case-sensitive`, and the library returns an error for
that combination.

Regular expressions use the regex engine's own case-sensitive semantics. The
plain `--case-sensitive` setting does not transform a regex.

## Pattern validation

Bitcoin and Solana exact patterns accept Base58 characters:

```text
123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz
```

Ethereum exact patterns accept ASCII hexadecimal characters. Invalid alphabet
characters fail before the search begins.

Regex patterns are also chain-validated. Alphanumeric literals must belong to
the chain's alphabet, and only this metacharacter set is accepted:

```text
^ $ . * + ? ( ) [ ] { } | -
```

The regex must then compile successfully. This restricted syntax intentionally
rejects escapes and other punctuation even if the underlying regex engine
would otherwise understand them.

## Empty and long patterns

The Rust API treats an empty exact or regex request as a request for one random
keypair. The CLI still requires a positional pattern or an input file.

Fast mode rejects exact patterns longer than the chain's fast limit. Use
`--disable-fast` only after estimating the work. Absolute limits remain 25
characters for Base58 chains and 40 characters for Ethereum.
