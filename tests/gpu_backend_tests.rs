#[cfg(feature = "gpu")]
mod gpu_tests {
    use std::str::FromStr;
    use std::time::{Duration, Instant};

    use bitcoin::Address;
    use bitcoin::hashes::{hash160, ripemd160, sha256, Hash};
    use bitcoin::Network::Bitcoin;
    use bitcoin::PublicKey;
    use btc_vanity::keys_and_address::BitcoinKeyPair;
    #[cfg(feature = "ethereum")]
    use btc_vanity::keys_and_address::EthereumKeyPair;
    #[cfg(any(feature = "ethereum", feature = "solana"))]
    use btc_vanity::keys_and_address::KeyPairGenerator;
    #[cfg(feature = "solana")]
    use btc_vanity::keys_and_address::SolanaKeyPair;
    use btc_vanity::vanity_addr_generator::chain::VanityChain;
    use btc_vanity::vanity_addr_generator::vanity_addr::GpuSearchTarget;
    use btc_vanity::vanity_addr_generator::gpu::{
        gpu_backend_available, Secp256k1GpuEngine, GPU_MAX_BATCH_SIZE,
    };
    use btc_vanity::{VanityAddr, VanityBackend, VanityMode, VanitySearchOptions};
    use num_bigint::BigUint;

    const SECP256K1_ORDER_HEX: &[u8] =
        b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141";

    #[test]
    fn test_gpu_bitcoin_addresses_match_cpu() {
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

    #[test]
    fn test_gpu_bitcoin_exact_search_matches_cpu_for_known_seed() {
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

        let mut gpu_match = None;
        for _ in 0..3 {
            let attempt = engine
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
            if attempt.is_some() {
                gpu_match = attempt;
                break;
            }
        }

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
                panic!(
                    "GPU exact search missed the known CPU address. seed={:02x?} gpu_key={:?} cpu={} debug_gpu={:?} cpu_compressed_pubkey={:02x?} gpu_compressed_pubkey={:02x?} cpu_sha256={} gpu_sha256={} cpu_hash={:?} gpu_hash={:?} opposite_parity={} zero_x={} prefix_zero_hash={}",
                    seed,
                    debug_match.as_ref().map(|value| value.private_key_bytes),
                    cpu.get_comp_address(),
                    debug_match.as_ref().map(|value| &value.address),
                    cpu_compressed_public_key,
                    gpu_debug.compressed_public_key,
                    cpu_sha256,
                    to_hex(&gpu_debug.sha256),
                    Address::from_str(cpu.get_comp_address())
                        .ok()
                        .and_then(|address| address.assume_checked().pubkey_hash())
                        .map(|hash| hash.to_string()),
                    Some(to_hex(&gpu_debug.hash160)),
                    opposite_parity_address(&cpu),
                    zero_x_address(&cpu),
                    prefix_zero_hash(&cpu),
                );
            }
        };

        assert_eq!(gpu_debug.private_key_bytes, seed);
        assert_eq!(
            gpu_debug.compressed_public_key,
            cpu_compressed_public_key,
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
        if !gpu_backend_available() {
            return;
        }

        let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");
        let seed = engine.generate_private_keys(1)[0];
        let cpu = <EthereumKeyPair as VanityChain>::from_private_key_bytes(seed)
            .expect("CPU derivation should succeed");

        let mut gpu_match = None;
        for _ in 0..3 {
            let attempt = engine
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
            if attempt.is_some() {
                gpu_match = attempt;
                break;
            }
        }

        let gpu_match = gpu_match.expect("GPU exact search should find the candidate");

        assert_eq!(gpu_match.private_key_bytes, seed);
        assert_eq!(gpu_match.address, *cpu.get_address());
        assert_eq!(gpu_match.attempts, 1);
    }

    #[cfg(feature = "solana")]
    #[test]
    fn test_gpu_solana_exact_search_matches_cpu_for_known_seed() {
        if !gpu_backend_available() {
            return;
        }

        let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");
        let seed = engine.generate_private_keys(1)[0];
        let cpu = <SolanaKeyPair as VanityChain>::from_private_key_bytes(seed)
            .expect("CPU derivation should succeed");
        let cpu_address = cpu.get_address();
        let pattern = &cpu_address.as_bytes()[..8];

        let mut gpu_match = None;
        for _ in 0..3 {
            let attempt = engine
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
            if attempt.is_some() {
                gpu_match = attempt;
                break;
            }
        }

        let gpu_match = gpu_match.expect("GPU exact search should find the candidate");

        assert_eq!(gpu_match.private_key_bytes, seed);
        assert_eq!(gpu_match.address, *cpu_address);
        assert_eq!(gpu_match.attempts, 1);
    }

    #[test]
    fn test_explicit_gpu_backend_finds_bitcoin_anywhere_match() {
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

    #[test]
    #[ignore = "benchmark test; run with `cargo test --features 'gpu ethereum solana' --test gpu_backend_tests gpu_end_to_end_cpu_vs_gpu_benchmarks -- --ignored --nocapture`"]
    fn gpu_end_to_end_cpu_vs_gpu_benchmarks() {
        if !gpu_backend_available() {
            eprintln!("Skipping GPU benchmark because no compatible adapter is available.");
            return;
        }

        let info_engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");
        println!("adapter={}", info_engine.adapter_name());
        drop(info_engine);

        let bitcoin_engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");

        run_chain_benchmarks::<BitcoinKeyPair>(
            &bitcoin_engine,
            "bitcoin",
            GpuSearchTarget::Bitcoin,
            b"zzzzzzzz",
        );

        #[cfg(feature = "ethereum")]
        let ethereum_engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");

        #[cfg(feature = "ethereum")]
        run_chain_benchmarks::<EthereumKeyPair>(
            &ethereum_engine,
            "ethereum",
            GpuSearchTarget::Ethereum,
            b"ffffffff",
        );

        #[cfg(feature = "solana")]
        let solana_engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");

        #[cfg(feature = "solana")]
        run_chain_benchmarks::<SolanaKeyPair>(
            &solana_engine,
            "solana",
            GpuSearchTarget::Solana,
            b"zzzzzzzz",
        );
    }

    fn run_chain_benchmarks<T: VanityChain + 'static>(
        engine: &Secp256k1GpuEngine,
        chain_name: &str,
        target: GpuSearchTarget,
        no_match_pattern: &[u8],
    ) {
        const SWEETSPOT_BATCH_CANDIDATES: [usize; 8] = [
            16_384,
            32_768,
            65_536,
            131_072,
            262_144,
            524_288,
            1_048_576,
            2_097_152,
        ];
        const CPU_BASELINE_ATTEMPTS_PER_RUN: usize = 65_536;

        let bench_run_cap = std::env::var("BENCH_RUNS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0);
        let max_config_secs = std::env::var("BENCH_MAX_CONFIG_SECS")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| *v > 0.0)
            .unwrap_or(12.0)
            .clamp(1.0, 30.0);
        let max_config_duration = Duration::from_secs_f64(max_config_secs);
        let bench_verbose = std::env::var("BENCH_VERBOSE")
            .ok()
            .map(|v| {
                let normalized = v.trim().to_ascii_lowercase();
                matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
            })
            .unwrap_or(true);

        let seed = engine.generate_private_keys(1)[0];

        for vanity_mode in [VanityMode::Prefix, VanityMode::Suffix, VanityMode::Anywhere] {
            let cpu_budget_start = Instant::now();
            let mut cpu_runs = Vec::new();
            let mut cpu_total_attempts = 0usize;
            let mut cpu_iterations = 0usize;
            while cpu_budget_start.elapsed() < max_config_duration {
                if let Some(cap) = bench_run_cap {
                    if cpu_iterations >= cap {
                        break;
                    }
                }

                let cpu_start = Instant::now();
                cpu_run_exact::<T>(
                    seed,
                    no_match_pattern,
                    false,
                    vanity_mode,
                    CPU_BASELINE_ATTEMPTS_PER_RUN,
                )
                .expect("CPU benchmark should succeed");
                cpu_runs.push(cpu_start.elapsed());
                cpu_total_attempts += CPU_BASELINE_ATTEMPTS_PER_RUN;
                cpu_iterations += 1;
            }

            if cpu_runs.is_empty() {
                let cpu_start = Instant::now();
                cpu_run_exact::<T>(
                    seed,
                    no_match_pattern,
                    false,
                    vanity_mode,
                    CPU_BASELINE_ATTEMPTS_PER_RUN,
                )
                .expect("CPU benchmark should succeed");
                cpu_runs.push(cpu_start.elapsed());
                cpu_total_attempts += CPU_BASELINE_ATTEMPTS_PER_RUN;
            }

            let cpu_median = median_duration(&cpu_runs);
            let cpu_best = *cpu_runs.iter().min().unwrap();
            let cpu_elapsed_total = cpu_budget_start.elapsed();
            let cpu_addr_per_s = cpu_total_attempts as f64
                / cpu_elapsed_total.as_secs_f64().max(f64::EPSILON);

            let mut best_batch = 0usize;
            let mut best_gpu_addr_per_s = 0.0f64;
            let mut no_gain_streak = 0usize;
            let mut best_high_batch_addr_per_s = 0.0f64;

            for batch_size in SWEETSPOT_BATCH_CANDIDATES {
                if batch_size > GPU_MAX_BATCH_SIZE {
                    continue;
                }

                let attempts_per_dispatch = batch_size * 2;
                let config_start = Instant::now();
                let mut gpu_runs = Vec::new();
                let mut gpu_total_attempts = 0usize;
                let mut gpu_iterations = 0usize;

                while config_start.elapsed() < max_config_duration {
                    if let Some(cap) = bench_run_cap {
                        if gpu_iterations >= cap {
                            break;
                        }
                    }

                    let gpu_start = Instant::now();
                    let _ = engine
                        .run_exact_batches(
                            target,
                            seed,
                            no_match_pattern,
                            false,
                            vanity_mode,
                            batch_size,
                            2,
                        )
                        .expect("GPU benchmark should succeed");
                    gpu_runs.push(gpu_start.elapsed());
                    gpu_total_attempts += attempts_per_dispatch;
                    gpu_iterations += 1;
                }

                if gpu_runs.is_empty() {
                    let gpu_start = Instant::now();
                    let _ = engine
                        .run_exact_batches(
                            target,
                            seed,
                            no_match_pattern,
                            false,
                            vanity_mode,
                            batch_size,
                            2,
                        )
                        .expect("GPU benchmark should succeed");
                    gpu_runs.push(gpu_start.elapsed());
                    gpu_total_attempts += attempts_per_dispatch;
                }

                let gpu_median = median_duration(&gpu_runs);
                let gpu_best = *gpu_runs.iter().min().unwrap();
                let gpu_elapsed_total = config_start.elapsed();
                let gpu_addr_per_s = gpu_total_attempts as f64
                    / gpu_elapsed_total.as_secs_f64().max(f64::EPSILON);
                let auto_choice = if gpu_addr_per_s > cpu_addr_per_s {
                    "gpu"
                } else {
                    "cpu"
                };

                if bench_verbose {
                    println!(
                        "{chain_name} mode={:?} batch={batch_size}: cpu median={:.3} ms best={:.3} ms {:.0} addr/s | gpu median={:.3} ms best={:.3} ms {:.0} addr/s | auto={auto_choice}",
                        vanity_mode,
                        cpu_median.as_secs_f64() * 1000.0,
                        cpu_best.as_secs_f64() * 1000.0,
                        cpu_addr_per_s,
                        gpu_median.as_secs_f64() * 1000.0,
                        gpu_best.as_secs_f64() * 1000.0,
                        gpu_addr_per_s,
                    );
                }

                if batch_size >= 262_144 {
                    best_high_batch_addr_per_s = best_high_batch_addr_per_s.max(gpu_addr_per_s);
                }

                if gpu_addr_per_s > best_gpu_addr_per_s * 1.03 {
                    best_gpu_addr_per_s = gpu_addr_per_s;
                    best_batch = batch_size;
                    no_gain_streak = 0;
                } else {
                    no_gain_streak += 1;
                }

                // Once throughput stops improving for two consecutive larger batches,
                // we've likely passed the practical sweet spot for this mode.
                if batch_size >= 262_144 && no_gain_streak >= 2 {
                    break;
                }
            }

            println!(
                "{chain_name} mode={:?} sweetspot batch={} gpu_throughput={:.0} addr/s high_batch_beats_cpu={} (gpu_high={:.0} cpu={:.0})",
                vanity_mode,
                best_batch,
                best_gpu_addr_per_s,
                best_high_batch_addr_per_s > cpu_addr_per_s,
                best_high_batch_addr_per_s,
                cpu_addr_per_s,
            );
        }
    }

    fn cpu_run_exact<T: VanityChain + 'static>(
        seed: [u8; 32],
        pattern: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
        attempts: usize,
    ) -> Result<(), btc_vanity::error::VanityError> {
        let lower_pattern = if case_sensitive {
            Vec::new()
        } else {
            pattern
                .iter()
                .map(|byte| byte.to_ascii_lowercase())
                .collect::<Vec<u8>>()
        };

        for offset in 0..attempts {
            let private_key = secp256k1_scalar_from_seed(seed, offset as u64);
            let candidate = T::from_private_key_bytes(private_key)?;
            let _ = matches_pattern(
                candidate.get_address_bytes(),
                pattern,
                &lower_pattern,
                case_sensitive,
                vanity_mode,
            );
        }

        Ok(())
    }

    fn matches_pattern(
        address_bytes: &[u8],
        pattern_bytes: &[u8],
        lower_pattern_bytes: &[u8],
        case_sensitive: bool,
        vanity_mode: VanityMode,
    ) -> bool {
        if case_sensitive {
            match vanity_mode {
                VanityMode::Prefix => address_bytes.starts_with(pattern_bytes),
                VanityMode::Suffix => address_bytes.ends_with(pattern_bytes),
                VanityMode::Anywhere => address_bytes.windows(pattern_bytes.len()).any(|w| w == pattern_bytes),
                VanityMode::Regex => unreachable!(),
            }
        } else {
            let address = address_bytes
                .iter()
                .map(|byte| byte.to_ascii_lowercase())
                .collect::<Vec<u8>>();
            match vanity_mode {
                VanityMode::Prefix => address.starts_with(lower_pattern_bytes),
                VanityMode::Suffix => address.ends_with(lower_pattern_bytes),
                VanityMode::Anywhere => address
                    .windows(lower_pattern_bytes.len())
                    .any(|window| window == lower_pattern_bytes),
                VanityMode::Regex => unreachable!(),
            }
        }
    }

    fn secp256k1_scalar_from_seed(seed: [u8; 32], offset: u64) -> [u8; 32] {
        let order =
            BigUint::parse_bytes(SECP256K1_ORDER_HEX, 16).expect("valid secp256k1 curve order");
        let seed_big = BigUint::from_bytes_be(&seed);
        let mut scalar = (seed_big + BigUint::from(offset)) % &order;
        if scalar == BigUint::default() {
            scalar = BigUint::from(1u8);
        }

        let bytes = scalar.to_bytes_be();
        let mut result = [0u8; 32];
        result[32 - bytes.len()..].copy_from_slice(&bytes);
        result
    }

    fn median_duration(samples: &[Duration]) -> Duration {
        let mut samples = samples.to_vec();
        samples.sort();
        samples[samples.len() / 2]
    }

    fn opposite_parity_address(keypair: &BitcoinKeyPair) -> String {
        let uncompressed = keypair.get_public_key().inner.serialize_uncompressed();
        let mut flipped = [0u8; 33];
        flipped[0] = if keypair.get_public_key().inner.serialize()[0] == 0x02 {
            0x03
        } else {
            0x02
        };
        flipped[1..].copy_from_slice(&uncompressed[1..33]);
        let secp_public_key = bitcoin::secp256k1::PublicKey::from_slice(&flipped).unwrap();
        Address::p2pkh(PublicKey::new(secp_public_key), Bitcoin).to_string()
    }

    fn zero_x_address(keypair: &BitcoinKeyPair) -> String {
        let prefix = keypair.get_public_key().inner.serialize()[0];
        let mut compressed = [0u8; 33];
        compressed[0] = prefix;
        match bitcoin::secp256k1::PublicKey::from_slice(&compressed) {
            Ok(secp_public_key) => Address::p2pkh(PublicKey::new(secp_public_key), Bitcoin).to_string(),
            Err(_) => "<invalid>".to_string(),
        }
    }

    fn prefix_zero_hash(keypair: &BitcoinKeyPair) -> String {
        let prefix = keypair.get_public_key().inner.serialize()[0];
        let mut compressed = [0u8; 33];
        compressed[0] = prefix;
        hash160::Hash::hash(&compressed).to_string()
    }

    fn to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
