//! Fixed-work throughput of the OpenCL Bitcoin prefix search.
//!
//! Each configuration runs a set number of dispatches against a pattern that
//! effectively never matches, so the figure is pure search throughput. Every
//! hit from a matching pattern is checked separately by
//! `tests/opencl_backend_tests.rs`.

use btc_vanity::opencl_btc::{benchmark_btc_prefix, opencl_device_available, OclConfig};

fn main() {
    if !opencl_device_available() {
        eprintln!("OpenCL benchmarks skipped: no OpenCL GPU device");
        return;
    }
    // Warm the driver and compiler cache.
    let _ = benchmark_btc_prefix("1zzzzzzzzz", true, OclConfig::default(), 2);

    println!(
        "{:>9} {:>6} {:>14} {:>9}",
        "items", "group", "keys", "Mkeys/s"
    );
    for (items, group) in [(16_384, 256), (32_768, 256), (65_536, 256), (65_536, 128)] {
        let cfg = OclConfig {
            items,
            group,
            ..OclConfig::default()
        };
        let stats = benchmark_btc_prefix("1zzzzzzzzz", true, cfg, 40).expect("benchmark runs");
        println!(
            "{:>9} {:>6} {:>14} {:>9.1}",
            items,
            group,
            stats.keys,
            stats.keys_per_second() / 1e6
        );
    }
}
