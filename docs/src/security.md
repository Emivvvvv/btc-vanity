# Security

btc-vanity generates private keys. The executable, random-number generation,
CPU or GPU path, terminal, output file, backups, and wallet import process all
belong to the secret-handling boundary.

## Before generating

- Use a reviewed release or revision and build it through a trusted toolchain.
- Run on a machine you control. Avoid shared shells, remote logging, screen
  recording, and untrusted monitoring software.
- Decide in advance how the result will move into the intended wallet.
- Use CPU if the local graphics device or driver is outside the boundary you
  are willing to trust.
- Test the complete workflow with a disposable, unfunded result.

No software can guarantee security merely by running locally. Operating-system
compromise, weak platform entropy, malicious dependencies, exposed output, and
operator mistakes remain relevant.

## Handle output as a secret

Without `--output-file`, private material appears in terminal scrollback. Shell
capture, terminal synchronization, clipboard history, and screenshots may
retain it.

With `--output-file`, output is appended in plaintext. New files request owner
read/write permissions on Unix, but existing permissions are preserved.
Network, removable, and non-Unix filesystems may apply different semantics.
Inspect the actual file permissions and backup behavior.

Do not place real generated keys in source control, issue reports, chat,
benchmark logs, test fixtures, or screenshots.

## Verify before funding

Independently derive the public address from the private material using trusted
wallet software. Compare the complete address, not only the vanity fragment.
First test signing and recovery with no value at risk.

Importing a key into a wallet exposes it to that wallet and its environment.
Use software whose key-import format matches the chain and output format.

## Backups and disposal

Keep the number of plaintext copies small. If a key will control value, use an
appropriate encrypted backup strategy with tested recovery. Deleting a file
does not necessarily remove copies from snapshots, cloud synchronization,
journals, swap, or storage media.

If a private key may have been exposed, do not rely on changing the vanity
pattern or file permissions afterward. Move assets to a newly generated,
unexposed wallet according to the chain's normal procedures.

## Scope

The project is distributed under the Apache License 2.0 without warranties.
Vanity generation changes address appearance only; it does not add authentication
or make a wallet safer.
