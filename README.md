# <img src='images/bitcoin.svg' height='22'> <img src='images/ethereum.png' height='22'> <img src='images/solana.png' height='22'> btc-vanity

A blazingly fast vanity address generator written with the Rust programming language. Supporting Bitcoin, Ethereum, and Solana.

With btc-vanity, you can generate wallets that has custom addresses with prefixes, suffixes, substrings, or even regex patterns. It's designed for **speed**, **flexibility**, and **security**.

You can easily run btc-vanity terminal application locally or use it as a library to create your vanity keypair securely.

## Key Features

**Multi-Chain Support**: Generate vanity addresses for: `Bitcoin`, `Ethereum`, and `Solana`. <br>
**Advanced Customization**: Match prefixes, suffixes, substrings, or regex-based patterns with optional case insensitivity. <br>
**Blazingly Fast Performance**: Fully utilize your hardware with customizable thread counts. <br>
**Batch File Support**: Bulk generate addresses using input files with desired patterns. <br>

## Installation

> [!CAUTION]
> btc-vanity has recently migrated to version 2. Please use it cautiously.

### CLI

Install the binary using `cargo`:

```bash
$ cargo install btc-vanity
```

### Library

Or include it as a library in your Rust project:

```toml
[dependencies]
btc-vanity = "2.0.1"
```
Crate on [crates.io](https://crates.io/crates/btc-vanity) <br>
Documentation on [docs.rs](https://docs.rs/btc-vanity/latest/btc_vanity/index.html)

## CLI Usage

### Basic CLI Syntax

The CLI tool provides several options to customize your address generation:

```shell
$ btc-vanity [OPTIONS] <PATTERN>
```

#### Blockchain Selection
`--btc`: Generates Bitcoin keypairs and addresses. [default] <br>
`--eth`: Generates Ethereum keypairs and addresses. <br>
`--sol`: Generates Solana keypairs and addresses. <br>

#### General Options
`-i, --input-file <FILE>`: Reads patterns and it's flags from the specified file for vanity address generation, with one pattern per line. <br>
`-o, --output-file <FILE>`: Saves generated wallet details to the specified file, creating it if it doesn’t exist or appending if it does. <br>
`-t, --threads <N>`: Sets the number of threads for address generation. <br>
`-f, --force-flags`: Forces CLI flags to override any flags specified in the input file, ensuring consistent behavior across all patterns. <br>
`-d, --disable-fast`: Disables fast mode to allow longer patterns (5 for BTC and SOL, 16 for ETH), though it may increase search time. <br>

#### Matching Options
`-p, --prefix`: Matches the pattern as a prefix of the address. [default] <br>
`-s, --suffix`: Matches the pattern as a suffix of the address. <br>
`-a, --anywhere`: Matches the pattern anywhere in the address. <br>
`-r, --regex <REGEX>`: Matches addresses using a regex pattern, supporting advanced customization like anchors and wildcards. <br>
`-c, --case-sensitive`: Enables case-sensitive matching, making patterns distinguish between uppercase and lowercase characters. <br>

### Bitcoin CLI Examples

Generate a Bitcoin address with prefix `1Emiv` (case-insensitive):

```shell
$ btc-vanity Emiv
```

Generate a Bitcoin address containing the substring `test` (case-sensitive):

```shell
$ btc-vanity -a -c test
```

Generate a Bitcoin address using a regex pattern `^1E.*T$`:

```shell
$ btc-vanity -r "^1E.*T$"
```

Generate multiple Bitcoin addresses and save to wallets.txt:

> [!NOTE]  
> -f flag will override any pattern flags inside the `input-file.txt`.
> For example if there line `emiv -s --eth` will become `emiv -p --btc -c`. 
> The resulting wallet will be printed in `wallets.txt`.

```shell
$ btc-vanity -f --btc -p -c -i input-file.txt -o wallets.txt
```

Generate an Ethereum address starting with 0xdead with 8 threads:

```shell
$ btc-vanity --eth -t 8 dead
```

Generate a Solana address ending with 123:

```shell
$ btc-vanity --sol -s 123
```

## Library Usage

Here are some usage examples of `btc-vanity` as a library.

### Generate a Bitcoin Vanity Address

Find a Bitcoin address that contains the substring `emiv` (case-insensitive) using 16 threads:

```rust
use btc_vanity::{BitcoinKeyPair, VanityAddr, VanityMode};

let vanity_address: BitcoinKeyPair = VanityAddr::generate(
            "emiv", // Desired substring
            16,     // Number of threads
            false,  // Case-insensitive
            true,   // Enable fast mode
            VanityMode::Anywhere // Match substring anywhere in the address
            ).unwrap();

println!("Vanity address:\n\
          private_key (WIF): {}\n\
          public_key (compressed): {}\n\
          address (compressed): {}\n",
          vanity_address.get_wif_private_key(),
          vanity_address.get_comp_public_key(),
          vanity_address.get_comp_address());
```

#### Generate an Ethereum Vanity Address

Match an Ethereum address with the prefix `0xdead` using 8 threads:

```rust
use btc_vanity::{EthereumKeyPair, KeyPairGenerator, VanityAddr, VanityMode};

let vanity_address: EthereumKeyPair = VanityAddr::generate(
            "dead", // Desired prefix (without 0x)
            8,      // Number of threads
            false,  // Case-insensitive (Case sensitivity not supported on ETH generation)
            true,   // Enable fast mode
            VanityMode::Prefix // Match substring at the start
            ).unwrap();

println!("Ethereum vanity address:\n\
          private_key: {}\n\
          public_key: {}\n\
          address: {}\n",
          vanity_address.get_private_key_as_hex(),
          vanity_address.get_private_key_as_hex(),
          vanity_address.get_address());
```

#### Generate a Solana Vanity Address

Create a Solana address with `meow` anywhere in the address (case-sensitive) using 4 threads:

```rust
use btc_vanity::{SolanaKeyPair, KeyPairGenerator, VanityAddr, VanityMode};

let vanity_address: SolanaKeyPair = VanityAddr::generate(
            "meow",  // Desired substring
            4,      // Number of threads
            true,  // Case-sensitive
            true,   // Enable fast mode
            VanityMode::Anywhere // Match substring anywhere in the address
            ).unwrap();

println!("Solana vanity address:\n\
          private_key: {}\n\
          public_key: {}\n\
          address: {}\n",
          vanity_address.get_private_key_as_base58(),
          vanity_address.get_public_key_as_base58(),
          vanity_address.get_address());
```

#### Regex Matching for Bitcoin Addresses

Find a Bitcoin address that matches a regex pattern `^1E.ET.*T$` with using 12 threads:

```rust
use btc_vanity::{BitcoinKeyPair, VanityAddr};

let vanity_address = VanityAddr::generate_regex::<BitcoinKeyPair>(
            "^1E.*ET.*T$", // The regex pattern
            12            // Number of threads
            ).unwrap();

println!("Bitcoin regex-matched vanity address:\n\
          private_key (WIF): {}\n\
          public_key (compressed): {}\n\
          address (compressed): {}\n",
          vanity_address.get_wif_private_key(),
          vanity_address.get_comp_public_key(),
          vanity_address.get_comp_address());
```

## Contributing

Contributions are welcome! If you’d like to improve btc-vanity or add support for additional chains, feel free to open an issue or submit a pull request on GitHub.

## Disclaimer

**USE WITH CAUTION AND UNDERSTANDING**

btc-vanity is a tool designed to assist users in generating customized vanity Bitcoin addresses using the Rust programming language. While btc-vanity aims to provide a secure and efficient method for generating vanity addresses, it is essential to exercise caution and follow the best security practices.

1. **Security Awareness**: Generating and using vanity addresses involves the creation of private keys and public addresses. Private keys grant control over the associated Bitcoin funds. It is crucial to understand the risks involved in managing private keys and to never share them with anyone. Keep your private keys stored securely and never expose them to potential threats.

2. **Risk of Loss**: Improper use of btc-vanity, mishandling of private keys, or failure to follow security guidelines may result in the loss of Bitcoin funds. Always double-check the addresses generated and verify their accuracy before using them for transactions.

3. **Verification**: Before utilizing any vanity address generated by btc-vanity, thoroughly verify the integrity of the software and the generated addresses. Only use versions of btc-vanity obtained from reputable sources, such as the official crates.io page.

4. **Backup and Recovery**: Maintain proper backups of your private keys and any relevant data. In the event of device failure, loss, or corruption, having secure backups will help prevent irreversible loss of funds.

5. **Use at Your Own Risk**: The btc-vanity software is provided "as is," without any warranties or guarantees. The author(s) and contributors of btc-vanity shall not be held responsible for any direct or indirect damages, losses, or liabilities resulting from the use or misuse of this software.

6. **Educational Purposes**: btc-vanity is intended for educational and personal use. It is your responsibility to ensure compliance with any legal, regulatory, or tax requirements in your jurisdiction related to Bitcoin and cryptocurrency usage.

By using btc-vanity, you acknowledge and accept the risks associated with generating vanity addresses and handling private keys. It is your responsibility to exercise diligence, follow security best practices, and be aware of potential risks.

Remember, the security of your Bitcoin holdings is paramount. Always prioritize the safety and security of your assets.