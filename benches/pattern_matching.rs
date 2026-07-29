use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn bench_pattern_matching(c: &mut Criterion) {
    let sample_btc_address = b"1EmivAddressTestStringKey123456789";

    let mut group = c.benchmark_group("pattern_matching");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(2));

    // Prefix matching benchmarks
    group.bench_function("btc_prefix_case_sensitive", |b| {
        b.iter(|| black_box(sample_btc_address.starts_with(b"1Emiv")))
    });
    group.bench_function("btc_prefix_case_insensitive", |b| {
        b.iter(|| {
            black_box(
                sample_btc_address
                    .get(..5)
                    .is_some_and(|p| p.eq_ignore_ascii_case(b"1emiv")),
            )
        })
    });

    // Anywhere matching benchmarks
    group.bench_function("btc_anywhere_case_sensitive", |b| {
        b.iter(|| black_box(sample_btc_address.windows(4).any(|w| w == b"Test")))
    });
    group.bench_function("btc_anywhere_case_insensitive", |b| {
        b.iter(|| {
            black_box(
                sample_btc_address
                    .windows(4)
                    .any(|w| w.eq_ignore_ascii_case(b"test")),
            )
        })
    });

    // Regex matching benchmark
    let re = regex::bytes::Regex::new(r"^1E.*T$").unwrap();
    group.bench_function("btc_regex_match", |b| {
        b.iter(|| black_box(re.is_match(sample_btc_address)))
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = bench_pattern_matching
}
criterion_main!(benches);
