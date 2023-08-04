// # Benchmark between old and new search engines.
//
// USE V0.9.0 TO BE ABLE TO RUN THIS TEST!
//
// As benchmark suggests new engine is a lot faster than the old one especially with string
// searches that longer than 1 character.
//
// Old engine's searches took 977.27 seconds total.
// New engine's searches took 627.29 seconds total
//
// Which means new engine is faster than the old one by ~1.58x (977.27 / 627.29)!
// (I think this is a better calculation than the function suggests (~1.64x))
//
// This test ran on a 8 cores m1 pro macbook pro 14 inch. Fans were on full blast mode.
//
//```bash
// $ cargo test --test benchmark -- --nocapture --test-threads=1
// ````
//
// ```bash
//    Compiling btc-vanity v0.9.0 (/Users/emivvvvv/Documents/GitHub/btc-vanity)
//     Finished test [optimized + debuginfo] target(s) in 0.15s
//      Running tests/benchmark.rs (target/debug/deps/benchmark-981a0dc8e71e6fdd)
//
// running 1 test
// test benchmarks ...
// Test settings ( threads: 16, case_sensititve: false, fast_mode: true, vanity_mode: Anywhere)
//
// test string: e, test count: 200000
// Finding 200000 vanity address took average: 163.35582925s with the old engine
// Finding 200000 vanity address took average: 145.740485167s with the new engine
// New engine is 1.1208678841902788x faster than the old one!
//
//
// test string: mi, test count: 8000
// Finding 8000 vanity address took average: 113.937177833s with the old engine
// Finding 8000 vanity address took average: 50.817601375s with the new engine
// New engine is 2.2420809867081215x faster than the old one!
//
//
// test string: vvv, test count: 1000
// Finding 1000 vanity address took average: 113.463775042s with the old engine
// Finding 1000 vanity address took average: 64.925782208s with the new engine
// New engine is 1.7475919609023864x faster than the old one!
//
//
// test string: Emiv, test count: 100
// Finding 100 vanity address took average: 445.538857334s with the old engine
// Finding 100 vanity address took average: 266.378989417s with the new engine
// New engine is 1.6725750717393713x faster than the old one!
//
//
// test string: 3169, test count: 10
// Finding 10 vanity address took average: 140.982839292s with the old engine
// Finding 10 vanity address took average: 99.419238167s with the new engine
// New engine is 1.4180639672090758x faster than the old one!
//
// Final result. New engine is 1.640235974149847x faster than the old one overall!
// ok
//
// test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1604.56s
// ```

#![allow(dead_code, unreachable_code, unused_variables)]
use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
use std::time::Instant;

const THREADS: u64 = 16;
const CASE_SENSITIVE: bool = false;
const FAST_MODE: bool = true;
const VANITY_MODE: VanityMode = VanityMode::Anywhere;

const TEST_STR_1: &str = "e";
const TEST_STR_2: &str = "mi";
const TEST_STR_3: &str = "vvv";
const TEST_STR_4: &str = "Emiv";
const TEST_STR_5: &str = "3169";

const TEST_COUNT_1: usize = 200000;
const TEST_COUNT_2: usize = 8000;
const TEST_COUNT_3: usize = 1000;
const TEST_COUNT_4: usize = 100;
const TEST_COUNT_5: usize = 10;

// USE V0.9.0 TO BE ABLE TO RUN THIS TEST!
fn benchmark(test_str: &str, test_count: usize) -> f64 {
    panic!("USE V0.9.0 TO BE ABLE TO RUN THIS TEST!");
    println!("\ntest string: {}, test count: {}", test_str, test_count);

    let start_engine_1 = Instant::now();
    for _ in 0..test_count {
        VanityAddr::generate(
            test_str,
            THREADS,
            CASE_SENSITIVE,
            FAST_MODE,
            VANITY_MODE,
            //false USE V0.9.0 TO BE ABLE TO RUN THIS TEST!
            ).unwrap();
    };
    let seconds_engine_1 = start_engine_1.elapsed().as_secs_f64();

    println!("Finding {} vanity address took average: {}s with the old engine", test_count, seconds_engine_1);

    let start_engine_2 = Instant::now();
    for _ in 0..test_count {
        VanityAddr::generate(
            test_str,
            THREADS,
            CASE_SENSITIVE,
            FAST_MODE,
            VANITY_MODE,
            //true USE V0.9.0 TO BE ABLE TO RUN THIS TEST!
        ).unwrap();
    };
    let seconds_engine_2 = start_engine_2.elapsed().as_secs_f64();

    let result = seconds_engine_1 / seconds_engine_2;
    println!("Finding {} vanity address took average: {}s with the new engine", test_count, seconds_engine_2);
    println!("New engine is {}x faster than the old one!\n", result);
    result
}

// USE V0.9.0 TO BE ABLE TO RUN THIS TEST!
//#[test] -- commenting this test because this test takes too long time!
fn benchmarks() {
    panic!("USE V0.9.0 TO BE ABLE TO RUN THIS TEST!");
    println!("\nTest settings ( threads: {}, case_sensititve: {}, fast_mode: {}, vanity_mode: {:?}) ",
             THREADS,
             CASE_SENSITIVE,
             FAST_MODE,
             VANITY_MODE,
    );

    let overall = (
            benchmark(TEST_STR_1, TEST_COUNT_1) + // New engine is 1.1208678841902788x faster than the old one!
            benchmark(TEST_STR_2, TEST_COUNT_2) + // New engine is 2.2420809867081215x faster than the old one!
            benchmark(TEST_STR_3, TEST_COUNT_3) + // New engine is 1.7475919609023864x faster than the old one!
            benchmark(TEST_STR_4, TEST_COUNT_4) + // New engine is 1.6725750717393713x faster than the old one!
            benchmark(TEST_STR_5, TEST_COUNT_5)   // New engine is 1.4180639672090758x faster than the old one!
            ) / 5.;                                                 // New engine is 1.640235974149847x faster than the old one overall!
    println!("Final result. New engine is {}x faster than the old one overall! ", overall)
}