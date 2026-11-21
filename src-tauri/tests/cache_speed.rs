use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokscale_ui_lib::data::cache::{parse_tui_cache_bytes, CacheOutcome};

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("tui-data-cache.v4.23.0.json")
}

#[test]
fn cold_parse_fixture_succeeds() {
    let bytes = std::fs::read(fixture_path()).expect("read v4.23.0 fixture");
    let start = Instant::now();
    let outcome = parse_tui_cache_bytes(&bytes);
    let elapsed = start.elapsed();

    assert!(
        matches!(outcome, CacheOutcome::Fresh(_)),
        "expected Fresh cache, got {outcome:?}"
    );
    assert!(
        elapsed < Duration::from_millis(50),
        "debug cold parse took unexpectedly long: {elapsed:?}"
    );
}

#[test]
#[ignore = "release-mode latency check; run with `cargo test --release -- --include-ignored`"]
fn cold_parse_of_v4_23_0_fixture_finishes_under_5ms_in_release() {
    let bytes = std::fs::read(fixture_path()).expect("read v4.23.0 fixture");
    let start = Instant::now();
    let outcome = parse_tui_cache_bytes(&bytes);
    let elapsed = start.elapsed();

    assert!(
        matches!(outcome, CacheOutcome::Fresh(_)),
        "expected Fresh cache, got {outcome:?}"
    );
    assert!(
        elapsed < Duration::from_millis(5),
        "cold parse exceeded 5 ms budget: {elapsed:?}"
    );
}
