# Your first search

Use a short, nonvaluable test search first:

```bash
btc-vanity --backend cpu --prefix abc
```

Bitcoin is the default chain and prefix is the default match mode. For Bitcoin
prefix search, btc-vanity accounts for the fixed leading `1`, so the command
looks for an address beginning with `1abc`.

The program reports the selected settings, searches, and then prints the
private key, public key, address, and elapsed time. The values are intentionally
not reproduced in this manual: every successful run creates fresh secret
material.

## Confirm the result without retaining it

For this first test, use a disposable result:

1. Check that the printed address has the requested prefix.
2. Do not send funds to it.
3. Clear terminal scrollback or close the disposable session when finished.

Terminal output contains a private key. Redirecting output, copying the
terminal, or recording the session creates additional secret copies.

## Try another match mode

```bash
btc-vanity --backend cpu --suffix abc
btc-vanity --backend cpu --anywhere abc
```

These modes search the complete encoded address. Prefix, suffix, and anywhere
are exact-pattern modes; regular expressions are a separate CPU-only mode.

## Try another chain

The corresponding feature must have been enabled during installation:

```bash
btc-vanity --eth --backend cpu --prefix abc
btc-vanity --sol --backend cpu --prefix abc
```

Ethereum patterns use hexadecimal digits and should omit `0x`. Solana patterns
use Base58. Read [Bitcoin, Ethereum, and Solana](chains.md) before comparing
their search costs.
