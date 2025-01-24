use btc_vanity::{BitcoinKeyPair, EthereumKeyPair, SolanaKeyPair, KeyPairGenerator, VanityMode, VanityAddr};

#[test]
fn test_bitcoin_vanity_address_prefix() {
    // Try generating a Bitcoin vanity address with a specific prefix
    let result = VanityAddr::generate::<BitcoinKeyPair>(
        "tst",          // Prefix
        8,               // Threads
        true,           // Case-insensitive
        true,            // Enable fast mode
        VanityMode::Prefix,
    );

    // Assert that the result is successful
    assert!(result.is_ok(), "Failed to generate Bitcoin vanity address");

    // Assert that the generated address starts with the prefix
    let keypair = result.unwrap();
    assert!(
        keypair.get_comp_address().starts_with("1tst"),
        "Generated address does not match prefix"
    );
}

#[test]
fn test_ethereum_vanity_address_prefix() {
    // Try generating an Ethereum vanity address with a specific prefix
    let result = VanityAddr::generate::<EthereumKeyPair>(
        "123",           // Prefix
        8,               // Threads
        true,            // Case-sensitive
        true,            // Enable fast mode
        VanityMode::Prefix,
    );

    // Assert that the result is successful
    assert!(result.is_ok(), "Failed to generate Ethereum vanity address");

    // Assert that the generated address starts with the prefix
    let keypair = result.unwrap();
    assert!(
        keypair.get_address_with_prefix().starts_with("0x123"),
        "Generated Ethereum address does not match prefix"
    );
}

#[test]
fn test_solana_vanity_address_prefix() {
    // Try generating a Solana vanity address with a specific prefix
    let result = VanityAddr::generate::<SolanaKeyPair>(
        "abc",           // Prefix
        8,               // Threads
        false,           // Case-insensitive
        false,           // Disable fast mode
        VanityMode::Prefix,
    );

    // Assert that the result is successful
    assert!(result.is_ok(), "Failed to generate Solana vanity address");

    // Assert that the generated address starts with the prefix
    let keypair = result.unwrap();
    assert!(
        keypair.get_address().to_ascii_lowercase().starts_with("abc"),
        "Generated Solana address does not match prefix"
    );
}

#[test]
fn test_bitcoin_vanity_address_regex() {
    // Regex to match addresses starting with "1A" and ending with "Z"
    let regex_pattern = "^1A.*Z$";

    let result = VanityAddr::generate_regex::<BitcoinKeyPair>(regex_pattern, 8);

    // Assert that the result is successful
    assert!(result.is_ok(), "Failed to generate Bitcoin vanity address with regex");

    // Assert that the generated address matches the regex
    let keypair = result.unwrap();
    let address = keypair.get_comp_address();
    let regex = regex::Regex::new(regex_pattern).unwrap();
    assert!(
        regex.is_match(&address),
        "Generated address '{}' does not match regex '{}'",
        address,
        regex_pattern
    );
}

#[test]
fn test_invalid_regex_handling() {
    // Invalid regex pattern
    let invalid_regex = "[";

    let result = VanityAddr::generate_regex::<BitcoinKeyPair>(invalid_regex, 8);

    // Assert that the result is an error
    assert!(result.is_err(), "Expected error for invalid regex, but got success");
}