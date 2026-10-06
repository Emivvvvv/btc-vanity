use btc_vanity::opencl_btc::{
    benchmark_btc_prefix, opencl_device_available, search_btc_prefix, OclConfig,
};

fn small_config() -> OclConfig {
    OclConfig {
        items: 4096,
        group: 64,
        ..OclConfig::default()
    }
}

#[test]
fn opencl_hits_all_verify_on_cpu() {
    if !opencl_device_available() {
        eprintln!("skipped: no OpenCL GPU device");
        return;
    }
    // A two-character prefix yields hundreds of hits per dispatch, each
    // re-derived on the CPU from the reconstructed private key.
    let stats = benchmark_btc_prefix("1ab", true, small_config(), 4).expect("benchmark runs");
    assert_eq!(stats.hash_mismatches, 0);
    assert!(stats.verified_hits > 0);
    let ratio = stats.verified_hits as f64 / stats.expected_hits;
    assert!(
        (0.8..1.2).contains(&ratio),
        "hit rate {ratio:.3} of expected (verified {}, expected {:.1})",
        stats.verified_hits,
        stats.expected_hits
    );
}

#[test]
fn opencl_case_insensitive_hits_verify() {
    if !opencl_device_available() {
        return;
    }
    let stats = benchmark_btc_prefix("zz", false, small_config(), 2).expect("benchmark runs");
    assert_eq!(stats.hash_mismatches, 0);
    assert!(stats.verified_hits > 0);
}

#[test]
fn opencl_search_returns_matching_wallet() {
    if !opencl_device_available() {
        return;
    }
    let pair = search_btc_prefix("1Ab", true, small_config()).expect("search finds a match");
    assert!(pair.get_comp_address().starts_with("1Ab"));
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let derived = bitcoin::PublicKey::from_private_key(&secp, pair.get_private_key());
    assert_eq!(&derived, pair.get_public_key());
}

#[test]
fn opencl_rejects_unsupported_patterns() {
    let cfg = small_config();
    assert!(search_btc_prefix("11abc", true, cfg).is_err());
    assert!(search_btc_prefix("1abc0", true, cfg).is_err());
}
