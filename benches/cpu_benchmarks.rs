use std::time::Duration;

use btc_vanity::keys_and_address::BitcoinKeyPair;
#[cfg(feature = "ethereum")]
use btc_vanity::keys_and_address::EthereumKeyPair;
#[cfg(feature = "solana")]
use btc_vanity::keys_and_address::SolanaKeyPair;
use btc_vanity::vanity_addr_generator::chain::VanityChain;
use btc_vanity::{VanityAddr, VanityBackend, VanityMode, VanitySearchOptions};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn cpu_benchmarks(c: &mut Criterion) {
    bench_cpu_chain::<BitcoinKeyPair>(c, "bitcoin", "1E");

    #[cfg(feature = "ethereum")]
    bench_cpu_chain::<EthereumKeyPair>(c, "ethereum", "d");

    #[cfg(feature = "solana")]
    bench_cpu_chain::<SolanaKeyPair>(c, "solana", "S");
}

fn bench_cpu_chain<T: VanityChain + 'static>(
    c: &mut Criterion,
    chain_name: &str,
    prefix_pattern: &str,
) {
    let mut group = c.benchmark_group(format!("cpu/multithreaded/{chain_name}"));
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(3));

    for threads in [1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("prefix_search", threads),
            &threads,
            |b, &threads| {
                b.iter(|| {
                    let options = VanitySearchOptions {
                        threads,
                        case_sensitive: false,
                        vanity_mode: VanityMode::Prefix,
                        backend: VanityBackend::Cpu,
                        ..VanitySearchOptions::default()
                    };
                    black_box(
                        VanityAddr::generate_with_options::<T>(prefix_pattern, options)
                            .expect("CPU search should succeed"),
                    )
                })
            },
        );
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().warm_up_time(Duration::from_secs(1));
    targets = cpu_benchmarks
}
criterion_main!(benches);
