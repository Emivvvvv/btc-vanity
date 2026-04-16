use std::time::Duration;

use btc_vanity::keys_and_address::BitcoinKeyPair;
#[cfg(feature = "ethereum")]
use btc_vanity::keys_and_address::EthereumKeyPair;
#[cfg(feature = "solana")]
use btc_vanity::keys_and_address::SolanaKeyPair;
use btc_vanity::vanity_addr_generator::chain::VanityChain;
use btc_vanity::vanity_addr_generator::gpu::{
    gpu_backend_available, Secp256k1GpuEngine, GPU_MAX_BATCH_SIZE,
};
use btc_vanity::vanity_addr_generator::vanity_addr::GpuSearchTarget;
use btc_vanity::VanityMode;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use num_bigint::BigUint;

const SECP256K1_ORDER_HEX: &[u8] =
    b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141";
const CPU_BASELINE_ATTEMPTS: usize = 65_536;
const GPU_BATCH_CANDIDATES: [usize; 6] = [32_768, 65_536, 131_072, 262_144, 524_288, 1_048_576];

fn gpu_end_to_end_benchmark(c: &mut Criterion) {
    if !gpu_backend_available() {
        return;
    }

    let engine = Secp256k1GpuEngine::new().expect("GPU engine should initialize");

    bench_chain::<BitcoinKeyPair>(c, &engine, "bitcoin", GpuSearchTarget::Bitcoin, b"zzzzzzzz");

    #[cfg(feature = "ethereum")]
    bench_chain::<EthereumKeyPair>(
        c,
        &engine,
        "ethereum",
        GpuSearchTarget::Ethereum,
        b"ffffffff",
    );

    #[cfg(feature = "solana")]
    bench_chain::<SolanaKeyPair>(c, &engine, "solana", GpuSearchTarget::Solana, b"zzzzzzzz");
}

fn bench_chain<T: VanityChain + 'static>(
    c: &mut Criterion,
    engine: &Secp256k1GpuEngine,
    chain_name: &str,
    target: GpuSearchTarget,
    no_match_pattern: &[u8],
) {
    let seed = engine.generate_private_keys(1)[0];

    for vanity_mode in [VanityMode::Prefix, VanityMode::Suffix, VanityMode::Anywhere] {
        let mut cpu_group = c.benchmark_group(format!("cpu_{chain_name}_{:?}", vanity_mode));
        cpu_group.sample_size(10);
        cpu_group.measurement_time(Duration::from_secs(6));
        cpu_group.throughput(Throughput::Elements(CPU_BASELINE_ATTEMPTS as u64));
        cpu_group.bench_function("scan", |b| {
            b.iter(|| {
                cpu_run_exact::<T>(
                    seed,
                    no_match_pattern,
                    false,
                    vanity_mode,
                    CPU_BASELINE_ATTEMPTS,
                )
                .expect("CPU benchmark should succeed")
            })
        });
        cpu_group.finish();

        let mut gpu_group = c.benchmark_group(format!("gpu_{chain_name}_{:?}", vanity_mode));
        gpu_group.sample_size(10);
        gpu_group.measurement_time(Duration::from_secs(6));

        for batch_size in GPU_BATCH_CANDIDATES {
            if batch_size > GPU_MAX_BATCH_SIZE {
                continue;
            }

            gpu_group.throughput(Throughput::Elements((batch_size * 2) as u64));
            gpu_group.bench_with_input(
                BenchmarkId::from_parameter(batch_size),
                &batch_size,
                |b, &batch| {
                    b.iter(|| {
                        black_box(
                            engine
                                .run_exact_batches(
                                    target,
                                    seed,
                                    no_match_pattern,
                                    false,
                                    vanity_mode,
                                    batch,
                                    2,
                                )
                                .expect("GPU benchmark should succeed"),
                        )
                    })
                },
            );
        }

        gpu_group.finish();
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
            VanityMode::Anywhere => address_bytes
                .windows(pattern_bytes.len())
                .any(|window| window == pattern_bytes),
            VanityMode::Regex => false,
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
