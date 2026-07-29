use std::time::Duration;

use btc_vanity::keys_and_address::BitcoinKeyPair;
#[cfg(feature = "ethereum")]
use btc_vanity::keys_and_address::EthereumKeyPair;
#[cfg(feature = "solana")]
use btc_vanity::keys_and_address::SolanaKeyPair;
use btc_vanity::vanity_addr_generator::chain::VanityChain;
use btc_vanity::vanity_addr_generator::gpu::{
    gpu_backend_available, GpuMatch, GpuTuning, Secp256k1GpuEngine, GPU_BATCH_SIZE,
    GPU_MAX_BATCH_SIZE,
};
use btc_vanity::vanity_addr_generator::vanity_addr::GpuSearchTarget;
use btc_vanity::VanityMode;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use num_bigint::BigUint;

const SECP256K1_ORDER_HEX: &[u8] =
    b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141";
const CPU_BASELINE_ATTEMPTS: usize = 16_384;
const GPU_BATCH_CANDIDATES: [usize; 1] = [GPU_BATCH_SIZE];

fn gpu_end_to_end_benchmark(c: &mut Criterion) {
    if !gpu_backend_available() {
        eprintln!("GPU benchmarks skipped: no compatible adapter is available");
        return;
    }

    bench_cold_gpu_initialization(c);

    let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");
    eprintln!("GPU steady-state adapter: {}", engine.adapter_name());

    bench_chain::<BitcoinKeyPair>(
        c,
        &engine,
        "bitcoin",
        GpuSearchTarget::Bitcoin,
        [b"1zzzzzzzz", b"zzzzzzzz", b"zzzzzzzz"],
        &[true],
    );

    #[cfg(feature = "ethereum")]
    bench_chain::<EthereumKeyPair>(
        c,
        &engine,
        "ethereum",
        GpuSearchTarget::Ethereum,
        [b"ffffffff", b"ffffffff", b"ffffffff"],
        &[true],
    );

    #[cfg(feature = "solana")]
    bench_chain::<SolanaKeyPair>(
        c,
        &engine,
        "solana",
        GpuSearchTarget::Solana,
        [b"zzzzzzzz", b"zzzzzzzz", b"zzzzzzzz"],
        &[true],
    );
}

fn bench_cold_gpu_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu/backend_gpu/state_cold_initialization");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(3));
    group.bench_function("create_engine", |b| {
        b.iter(|| {
            let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");
            black_box(engine.adapter_name())
        })
    });
    group.finish();
}

fn bench_chain<T: VanityChain + 'static>(
    c: &mut Criterion,
    engine: &Secp256k1GpuEngine,
    chain_name: &str,
    target: GpuSearchTarget,
    patterns: [&[u8]; 3],
    case_modes: &[bool],
) {
    let seed = engine.generate_private_keys(1)[0];

    for (mode_index, vanity_mode) in [VanityMode::Prefix, VanityMode::Suffix, VanityMode::Anywhere]
        .into_iter()
        .enumerate()
    {
        let pattern = patterns[mode_index];

        for &case_sensitive in case_modes {
            let case_label = if case_sensitive {
                "case_sensitive"
            } else {
                "case_insensitive"
            };

            let mut cpu_group = c.benchmark_group(format!(
                "steady/backend_cpu/chain_{chain_name}/mode_{vanity_mode:?}/{case_label}"
            ));
            cpu_group.sample_size(10);
            cpu_group.measurement_time(Duration::from_secs(3));
            cpu_group.throughput(Throughput::Elements(CPU_BASELINE_ATTEMPTS as u64));
            cpu_group.bench_function("scan", |b| {
                b.iter(|| {
                    cpu_run_exact::<T>(
                        seed,
                        pattern,
                        case_sensitive,
                        vanity_mode,
                        CPU_BASELINE_ATTEMPTS,
                    )
                    .expect("CPU benchmark should succeed")
                })
            });
            cpu_group.finish();

            let tuning = GpuTuning::default();
            let mut gpu_group = c.benchmark_group(format!(
                "steady/backend_gpu/chain_{chain_name}/mode_{vanity_mode:?}/{case_label}/ring_{}/candidates_{}",
                tuning.ring_depth, tuning.candidates_per_invocation
            ));
            gpu_group.sample_size(10);
            gpu_group.measurement_time(Duration::from_secs(3));

            for batch_size in GPU_BATCH_CANDIDATES {
                if batch_size > GPU_MAX_BATCH_SIZE {
                    continue;
                }

                let tuning = GpuTuning {
                    batch_size,
                    ..tuning
                };
                let batches = tuning.ring_depth;
                gpu_group.throughput(Throughput::Elements(tuning.attempts_for_batches(batches)));
                gpu_group.bench_with_input(
                    BenchmarkId::new("batch", batch_size),
                    &batch_size,
                    |b, &batch| {
                        b.iter(|| {
                            let result = engine
                                .run_exact_batches(
                                    target,
                                    seed,
                                    pattern,
                                    case_sensitive,
                                    vanity_mode,
                                    batch,
                                    batches,
                                )
                                .expect("GPU benchmark should succeed");
                            validate_gpu_result::<T>(&result);
                            black_box(result)
                        })
                    },
                );
            }

            gpu_group.finish();
        }
    }
}

fn validate_gpu_result<T: VanityChain>(result: &Option<GpuMatch>) {
    if let Some(found) = result {
        let reconstructed = T::from_private_key_bytes(found.private_key_bytes)
            .expect("GPU benchmark returned an invalid private key");
        assert_eq!(
            reconstructed.get_address(),
            &found.address,
            "GPU benchmark returned a mismatched key/address pair"
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
        black_box(matches_pattern(
            candidate.get_address_bytes(),
            pattern,
            &lower_pattern,
            case_sensitive,
            vanity_mode,
        ));
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
            VanityMode::Anywhere => address_bytes
                .windows(pattern_bytes.len())
                .any(|window| window == pattern_bytes),
            VanityMode::Regex => false,
        }
    } else {
        match vanity_mode {
            VanityMode::Prefix => address_bytes
                .get(..lower_pattern_bytes.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(lower_pattern_bytes)),
            VanityMode::Suffix => address_bytes
                .len()
                .checked_sub(lower_pattern_bytes.len())
                .and_then(|start| address_bytes.get(start..))
                .is_some_and(|suffix| suffix.eq_ignore_ascii_case(lower_pattern_bytes)),
            VanityMode::Anywhere => address_bytes
                .windows(lower_pattern_bytes.len())
                .any(|window| window.eq_ignore_ascii_case(lower_pattern_bytes)),
            VanityMode::Regex => false,
        }
    }
}

fn secp256k1_scalar_from_seed(seed: [u8; 32], offset: u64) -> [u8; 32] {
    let order = BigUint::parse_bytes(SECP256K1_ORDER_HEX, 16).expect("valid secp256k1 order");
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

criterion_group! {
    name = benches;
    config = Criterion::default().warm_up_time(Duration::from_secs(2));
    targets = gpu_end_to_end_benchmark
}
criterion_main!(benches);
