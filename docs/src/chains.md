# Bitcoin, Ethereum, and Solana

The three chains use different curves, address pipelines, alphabets, and
display conventions. A pattern valid on one chain may be impossible on another.

| Chain | Curve | Address text in this project | Cargo feature |
| --- | --- | --- | --- |
| Bitcoin | secp256k1 | Mainnet P2PKH Base58Check, starts with `1` | built in |
| Ethereum | secp256k1 | 40 lowercase hexadecimal digits; CLI adds `0x` | `ethereum` |
| Solana | Ed25519 | Base58-encoded public key | `solana` |

## Bitcoin

btc-vanity generates a secp256k1 private key, derives a compressed public key,
and produces a mainnet pay-to-public-key-hash (P2PKH) address. Prefix input does
not include the fixed leading `1`:

```bash
btc-vanity --btc --prefix Ab
```

This searches for `1Ab` by default with ASCII case-insensitive comparison.
Bitcoin exact patterns must use the Base58 alphabet. `0`, `O`, `I`, and `l` are
not valid Base58 characters.

## Ethereum

btc-vanity derives an uncompressed secp256k1 public key, hashes the 64-byte
coordinates, and uses the last 20 bytes as the address. Internally the address
is 40 lowercase hex digits without a prefix. The CLI adds `0x` when displaying
it.

```bash
btc-vanity --eth --prefix dead
```

Patterns must contain only `0-9`, `a-f`, or `A-F` and must omit `0x`. Ethereum
plain matching is case-insensitive; `--case-sensitive` is rejected. The project
does not generate mixed-case checksum display addresses.

## Solana

btc-vanity creates a 32-byte seed, derives an Ed25519 keypair, and encodes the
32-byte public key with Base58:

```bash
btc-vanity --sol --suffix Ab
```

Solana has no fixed leading character. Its exact patterns use the same Base58
alphabet validation as Bitcoin, and matching is case-insensitive unless
`--case-sensitive` is set.

## Length limits

Fast mode is enabled by default as a guard against accidentally requesting a
very long search:

| Chain | Fast-mode maximum | Absolute maximum |
| --- | ---: | ---: |
| Bitcoin | 5 pattern characters | 25 |
| Ethereum | 16 hexadecimal characters | 40 |
| Solana | 5 pattern characters | 25 |

`--disable-fast` removes the fast-mode limit, not the absolute limit. Passing
validation does not imply that a search is practical. Even much shorter
patterns can require substantial time.
