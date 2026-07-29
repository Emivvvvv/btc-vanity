# What is a vanity address?

A cryptocurrency wallet starts with private key material. Cryptographic curve
operations derive a public key from it, and the chain's address format derives
the address from that public key.

The private key authorizes spending or signing. The public key and address are
safe to share for their intended purpose; the private key is not. Anyone who
obtains a generated private key may be able to control assets associated with
it.

## Why the address cannot be edited

An address is not a profile name stored beside a key. It is the deterministic
output of cryptographic operations and an encoding pipeline. Changing one
visible character would ordinarily produce a string that no longer corresponds
to the original public key, or that fails the format's checksum.

Vanity generation therefore works by brute force:

1. Generate candidate private key material.
2. Derive its public key.
3. Derive and encode the address.
4. Compare the address with the requested pattern.
5. Keep the first matching keypair and discard the other candidates.

The search does not weaken or reverse the address pipeline. It searches for a
keypair whose normal address happens to have the desired text.

## Why difficulty grows quickly

Suppose each requested character has `A` plausible values. A particular
`n`-character prefix then has probability around `1 / A^n` per candidate, and
needs around `A^n` candidates on average. The exact probability depends on the
chain, position, case rules, and encoding, but the exponential shape remains.

Bitcoin and Solana use the Base58 alphabet, which removes visually ambiguous
characters such as `0`, `O`, `I`, and `l`. Ethereum addresses in this project
are hexadecimal, so each digit has 16 possible values. These alphabets are not
interchangeable: a valid hexadecimal pattern can be invalid Base58 and vice
versa.

Prefix and suffix constraints are generally rarer than an equally long
substring, because a substring has several possible positions. Case-insensitive
matching may also accept more candidates than case-sensitive matching.

## What a vanity address does not provide

A memorable address is not proof of identity, ownership, or trustworthy
software. It does not make a transaction safer, add a password, or protect the
private key. Verify the full address through a trusted channel; similar-looking
addresses remain a common source of mistakes.
