# Chain pipelines

Search performance and correctness depend on the complete address pipeline.
The curve operation alone is not an address.

## Bitcoin P2PKH

For each Bitcoin candidate:

1. Interpret the private scalar on secp256k1.
2. Derive and serialize the compressed 33-byte public key.
3. Compute SHA-256 of the compressed public key.
4. Compute RIPEMD-160 of that SHA-256 digest.
5. Prepend the Bitcoin mainnet P2PKH version byte.
6. Add the four-byte checksum derived from double SHA-256.
7. Base58-encode the versioned payload and checksum (Base58Check).

The result starts with `1`. This project searches legacy mainnet P2PKH
addresses; it does not search Bech32 or script-hash formats.

## Ethereum

For each Ethereum candidate:

1. Interpret the private scalar on secp256k1.
2. Derive the uncompressed 65-byte public key.
3. Remove the leading uncompressed-key marker byte.
4. Compute Keccak-256 over the remaining 64 bytes.
5. Take the final 20 bytes.
6. Encode them as 40 lowercase hexadecimal digits.

The library stores the address without `0x`; the CLI adds `0x` when printing.
Keccak-256 is the Ethereum hash variant, not standardized SHA3-256. This project
does not add mixed-case checksum capitalization.

## Solana

For each Solana candidate:

1. Begin with a 32-byte seed.
2. Derive the Ed25519 keypair and 32-byte public key.
3. Base58-encode the public key.

The public-key text is the address. Unlike Bitcoin, Solana does not wrap it in a
versioned Base58Check payload.

## Base58 and hexadecimal are display encodings

Base58 represents bytes using a 58-character alphabet chosen to avoid ambiguous
glyphs. Hexadecimal represents each four bits with one of 16 digits. Neither
encoding makes an address editable: decoding, changing bytes, and re-encoding
would describe different data and would not preserve the original keypair.

The different alphabets also produce different per-character search
probabilities. Compare measured candidate rates and target probabilities per
chain rather than transferring an estimate from one format to another.
