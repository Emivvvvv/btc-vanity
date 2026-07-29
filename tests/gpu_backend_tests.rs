#[cfg(feature = "gpu")]
mod gpu_tests {
    use bitcoin::hashes::{ripemd160, sha256, Hash};
    use btc_vanity::keys_and_address::BitcoinKeyPair;
    #[cfg(feature = "ethereum")]
    use btc_vanity::keys_and_address::EthereumKeyPair;
    #[cfg(any(feature = "ethereum", feature = "solana"))]
    use btc_vanity::keys_and_address::KeyPairGenerator;
    #[cfg(feature = "solana")]
    use btc_vanity::keys_and_address::SolanaKeyPair;
    use btc_vanity::vanity_addr_generator::chain::VanityChain;
    use btc_vanity::vanity_addr_generator::gpu::{gpu_backend_available, Secp256k1GpuEngine};
    use btc_vanity::vanity_addr_generator::vanity_addr::GpuSearchTarget;
    use btc_vanity::{VanityAddr, VanityBackend, VanityMode, VanitySearchOptions};
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn gpu_test_guard() -> MutexGuard<'static, ()> {
        static GPU_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        GPU_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn test_gpu_bitcoin_addresses_match_cpu() {
        let _guard = gpu_test_guard();
        if !gpu_backend_available() {
            return;
        }

        let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");
        let private_keys = engine.generate_private_keys(32);
        let public_keys = engine
            .derive_public_keys(&private_keys)
            .expect("GPU derivation should succeed");

        for (private_key, public_key) in private_keys.into_iter().zip(public_keys) {
            let cpu = <BitcoinKeyPair as VanityChain>::from_private_key_bytes(private_key)
                .expect("CPU derivation should succeed");
            let gpu_address =
                <BitcoinKeyPair as VanityChain>::address_from_gpu_public_key(&public_key)
                    .expect("GPU public key should map to a Bitcoin address");

            assert_eq!(cpu.get_comp_address(), &gpu_address);
        }
    }

    #[cfg(all(feature = "ethereum", feature = "solana"))]
    #[test]
    fn test_gpu_result_claim_has_one_valid_writer() {
        let _guard = gpu_test_guard();
        if !gpu_backend_available() {
            eprintln!("skipping GPU race test: no supported adapter");
            return;
        }

        const REPETITIONS: usize = 16;
        const MULTI_WORKGROUP_BATCH: usize = 8_192;

        let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");

        for _ in 0..REPETITIONS {
            let seed = engine.generate_private_keys(1)[0];
            let found = engine
                .run_exact_batches(
                    GpuSearchTarget::Bitcoin,
                    seed,
                    b"",
                    true,
                    VanityMode::Prefix,
                    MULTI_WORKGROUP_BATCH,
                    1,
                )
                .expect("Bitcoin GPU race search should succeed")
                .expect("empty Bitcoin pattern should produce a winner");
            let reconstructed =
                <BitcoinKeyPair as VanityChain>::from_private_key_bytes(found.private_key_bytes)
                    .expect("GPU should return a valid Bitcoin private key");
            assert_eq!(reconstructed.get_comp_address(), &found.address);
        }

        for _ in 0..REPETITIONS {
            let seed = engine.generate_private_keys(1)[0];
            let found = engine
                .run_exact_batches(
                    GpuSearchTarget::Ethereum,
                    seed,
                    b"",
                    true,
                    VanityMode::Prefix,
                    MULTI_WORKGROUP_BATCH,
                    1,
                )
                .expect("Ethereum GPU race search should succeed")
                .expect("empty Ethereum pattern should produce a winner");
            let reconstructed =
                <EthereumKeyPair as VanityChain>::from_private_key_bytes(found.private_key_bytes)
                    .expect("GPU should return a valid Ethereum private key");
            assert_eq!(reconstructed.get_address(), &found.address);
        }

        for _ in 0..REPETITIONS {
            let seed = engine.generate_private_keys(1)[0];
            let found = engine
                .run_exact_batches(
                    GpuSearchTarget::Solana,
                    seed,
                    b"",
                    true,
                    VanityMode::Prefix,
                    MULTI_WORKGROUP_BATCH,
                    1,
                )
                .expect("Solana GPU race search should succeed")
                .expect("empty Solana pattern should produce a winner");
            let reconstructed =
                <SolanaKeyPair as VanityChain>::from_private_key_bytes(found.private_key_bytes)
                    .expect("GPU should return a valid Solana private key");
            assert_eq!(reconstructed.get_address(), &found.address);
        }
    }

    #[test]
    fn test_gpu_bitcoin_exact_search_matches_cpu_for_known_seed() {
        let _guard = gpu_test_guard();
        if !gpu_backend_available() {
            return;
        }

        let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");
        let seed = engine.generate_private_keys(1)[0];
        let cpu = <BitcoinKeyPair as VanityChain>::from_private_key_bytes(seed)
            .expect("CPU derivation should succeed");
        let cpu_compressed_public_key = cpu.get_public_key().inner.serialize();
        let cpu_sha256 = sha256::Hash::hash(&cpu_compressed_public_key);
        let cpu_hash160 = ripemd160::Hash::hash(cpu_sha256.as_byte_array());
        let gpu_debug = engine
            .debug_bitcoin_candidate(seed)
            .expect("GPU debug search should succeed")
            .expect("debug search should always match a Bitcoin address");

        let gpu_match = engine
            .run_exact_batches(
                GpuSearchTarget::Bitcoin,
                seed,
                cpu.get_comp_address().as_bytes(),
                true,
                VanityMode::Prefix,
                1,
                1,
            )
            .expect("GPU exact search should succeed");

        let gpu_match = match gpu_match {
            Some(gpu_match) => gpu_match,
            None => {
                let debug_match = engine
                    .run_exact_batches(
                        GpuSearchTarget::Bitcoin,
                        seed,
                        b"1",
                        true,
                        VanityMode::Anywhere,
                        1,
                        1,
                    )
                    .expect("debug GPU exact search should succeed");

                let gpu_hex_hash = debug_match
                    .as_ref()
                    .and_then(|m| m.debug_hash.as_ref())
                    .map(|h| to_hex(&h[..20]));

                panic!(
                    "GPU exact search missed the known CPU address.\nseed={:02x?}\ngpu_key={:?}\ncpu={}\ngpu_addr={:?}\ncpu_hash={}\ngpu_hash={:?}",
                    seed,
                    debug_match.as_ref().map(|value| value.private_key_bytes),
                    cpu.get_comp_address(),
                    debug_match.as_ref().map(|value| &value.address),
                    to_hex(cpu_hash160.as_byte_array()),
                    gpu_hex_hash,
                );
            }
        };

        assert_eq!(gpu_debug.private_key_bytes, seed);
        assert_eq!(
            gpu_debug.compressed_public_key, cpu_compressed_public_key,
            "GPU compressed public key should match CPU"
        );
        assert_eq!(
            gpu_debug.sha256.as_slice(),
            cpu_sha256.as_byte_array(),
            "GPU SHA-256(pubkey) should match CPU"
        );
        assert_eq!(
            gpu_debug.hash160.as_slice(),
            cpu_hash160.as_byte_array(),
            "GPU HASH160(pubkey) should match CPU"
        );
        assert_eq!(gpu_debug.address, *cpu.get_comp_address());
        assert_eq!(gpu_match.private_key_bytes, seed);
        assert_eq!(gpu_match.address, *cpu.get_comp_address());
        assert_eq!(gpu_match.attempts, 1);
    }

    #[cfg(feature = "ethereum")]
    #[test]
    fn test_gpu_ethereum_exact_search_matches_cpu_for_known_seed() {
        let _guard = gpu_test_guard();
        if !gpu_backend_available() {
            return;
        }

        let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");
        let seed = [
            0xef, 0x41, 0x60, 0x22, 0x4f, 0x6e, 0xd7, 0x6f, 0x13, 0xb4, 0x97, 0xf8, 0x1d, 0x46,
            0x4b, 0x57, 0x8d, 0x14, 0x4c, 0xef, 0x79, 0xe4, 0x6f, 0xfc, 0x4a, 0x62, 0xa6, 0x0b,
            0x58, 0x91, 0x4e, 0x13,
        ];
        let cpu = <EthereumKeyPair as VanityChain>::from_private_key_bytes(seed)
            .expect("CPU derivation should succeed");

        let gpu_match = engine
            .run_exact_batches(
                GpuSearchTarget::Ethereum,
                seed,
                cpu.get_address().as_bytes(),
                true,
                VanityMode::Prefix,
                1,
                1,
            )
            .expect("GPU exact search should succeed");

        let gpu_match = match gpu_match {
            Some(m) => m,
            None => {
                let debug_match = engine
                    .run_exact_batches(
                        GpuSearchTarget::Ethereum,
                        seed,
                        b"0",
                        true,
                        VanityMode::Anywhere,
                        1,
                        1,
                    )
                    .expect("debug GPU ETH search should succeed");
                let gpu_hex_hash = debug_match
                    .as_ref()
                    .and_then(|m| m.debug_hash.as_ref())
                    .map(|h| to_hex(&h[..32]));
                panic!(
                    "GPU Ethereum search missed known address.\nseed={:02x?}\ncpu={}\ngpu_addr={:?}\ngpu_hash={:?}",
                    seed,
                    cpu.get_address(),
                    debug_match.as_ref().map(|m| &m.address),
                    gpu_hex_hash,
                );
            }
        };

        assert_eq!(gpu_match.private_key_bytes, seed);
        assert_eq!(gpu_match.address, *cpu.get_address());
        assert_eq!(gpu_match.attempts, 1);
    }

    #[cfg(feature = "solana")]
    #[test]
    fn test_gpu_solana_exact_search_matches_cpu_for_known_seed() {
        let _guard = gpu_test_guard();
        if !gpu_backend_available() {
            return;
        }

        let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");
        let seed = engine.generate_private_keys(1)[0];
        let cpu = <SolanaKeyPair as VanityChain>::from_private_key_bytes(seed)
            .expect("CPU derivation should succeed");
        let cpu_address = cpu.get_address();
        let pattern = &cpu_address.as_bytes()[..8];

        let gpu_match = engine
            .run_exact_batches(
                GpuSearchTarget::Solana,
                seed,
                pattern,
                true,
                VanityMode::Prefix,
                1,
                1,
            )
            .expect("GPU exact search should succeed");

        let gpu_match = match gpu_match {
            Some(m) => m,
            None => {
                let debug_match = engine
                    .run_exact_batches(
                        GpuSearchTarget::Solana,
                        seed,
                        b"1",
                        true,
                        VanityMode::Anywhere,
                        1,
                        1,
                    )
                    .expect("debug GPU SOL search should succeed");
                let gpu_hex_hash = debug_match
                    .as_ref()
                    .and_then(|m| m.debug_hash.as_ref())
                    .map(|h| to_hex(&h[..32]));
                panic!(
                    "GPU Solana search missed known address.\nseed={:02x?}\ncpu={}\ngpu_addr={:?}\ngpu_hash={:?}",
                    seed,
                    cpu_address,
                    debug_match.as_ref().map(|m| &m.address),
                    gpu_hex_hash,
                );
            }
        };

        assert_eq!(gpu_match.private_key_bytes, seed);
        assert_eq!(gpu_match.address, *cpu_address);
        assert_eq!(gpu_match.attempts, 1);
    }

    #[test]
    fn test_throttled_gpu_backend_finds_bitcoin_anywhere_match() {
        let _guard = gpu_test_guard();
        if !gpu_backend_available() {
            return;
        }

        let result = VanityAddr::generate_with_options::<BitcoinKeyPair>(
            "1",
            VanitySearchOptions {
                threads: 1,
                case_sensitive: true,
                fast_mode: true,
                vanity_mode: VanityMode::Anywhere,
                backend: VanityBackend::Gpu,
                gpu_batch_size: None,
                gpu_usage_limit: Some(70),
            },
        )
        .expect("explicit GPU backend should generate a matching Bitcoin address");

        assert!(
            result.get_comp_address().contains('1'),
            "generated address should contain 1, got {}",
            result.get_comp_address()
        );
    }

    #[test]
    fn test_explicit_hybrid_backend_finds_bitcoin_anywhere_match() {
        let _guard = gpu_test_guard();
        if !gpu_backend_available() {
            return;
        }

        let result = VanityAddr::generate_with_options::<BitcoinKeyPair>(
            "1",
            VanitySearchOptions {
                threads: 2,
                case_sensitive: true,
                fast_mode: true,
                vanity_mode: VanityMode::Anywhere,
                backend: VanityBackend::Hybrid,
                gpu_batch_size: None,
                gpu_usage_limit: Some(100),
            },
        )
        .expect("explicit hybrid backend should generate a matching Bitcoin address");

        assert!(
            result.get_comp_address().contains('1'),
            "generated address should contain 1, got {}",
            result.get_comp_address()
        );
    }

    #[test]
    fn test_gpu_regex_is_rejected_but_auto_regex_uses_cpu() {
        let _guard = gpu_test_guard();
        let gpu_result = VanityAddr::generate_regex_with_options::<BitcoinKeyPair>(
            "^1",
            VanitySearchOptions {
                threads: 1,
                vanity_mode: VanityMode::Regex,
                backend: VanityBackend::Gpu,
                ..VanitySearchOptions::default()
            },
        );

        assert!(matches!(
            gpu_result,
            Err(btc_vanity::error::VanityError::GpuRegexUnsupported)
        ));

        let hybrid_result = VanityAddr::generate_regex_with_options::<BitcoinKeyPair>(
            "^1",
            VanitySearchOptions {
                threads: 1,
                vanity_mode: VanityMode::Regex,
                backend: VanityBackend::Hybrid,
                ..VanitySearchOptions::default()
            },
        )
        .expect("hybrid regex should fall back to CPU");

        assert!(hybrid_result.get_comp_address().starts_with('1'));

        let auto_result = VanityAddr::generate_regex_with_options::<BitcoinKeyPair>(
            "^1",
            VanitySearchOptions {
                threads: 1,
                vanity_mode: VanityMode::Regex,
                backend: VanityBackend::Auto,
                ..VanitySearchOptions::default()
            },
        )
        .expect("auto regex should fall back to CPU");

        assert!(auto_result.get_comp_address().starts_with('1'));
    }

    fn to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
