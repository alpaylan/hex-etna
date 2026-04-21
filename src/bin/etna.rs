// ETNA workload runner for hex.
//
// Usage: cargo run --release --bin etna -- <tool> <property>
//   tool:     etna | proptest | quickcheck | crabcheck | hegel
//   property: FromHexRejectsWhitespace | InvalidCharErrorDisplayEscaped | All
//
// Each invocation emits a single JSON line on stdout and exits 0
// (usage errors exit 2).

use crabcheck::quickcheck as crabcheck_qc;
use crabcheck::quickcheck::Arbitrary as CcArbitrary;
use hegel::{generators as hgen, Hegel, Settings as HegelSettings};
use hex::etna::{
    property_from_hex_rejects_whitespace, property_invalid_char_error_display_escaped,
    PropertyResult,
};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestError, TestRunner};
use quickcheck::{Arbitrary as QcArbitrary, Gen, QuickCheck, ResultStatus, TestResult};
use rand::Rng;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

// Shared wrapper for arbitrary byte vectors. Vec<u8> alone lacks Display,
// which quickcheck's counterexample rendering requires; we also need
// identical-shape generators for quickcheck, crabcheck, and proptest.
#[derive(Clone)]
struct HexBytes(Vec<u8>);

impl fmt::Debug for HexBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for HexBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl QcArbitrary for HexBytes {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = g.random_range(0..64u32) as usize;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(g.random_range(0..=u8::MAX));
        }
        HexBytes(v)
    }
}

impl<R: Rng> CcArbitrary<R> for HexBytes {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let len = rng.random_range(0..64u32) as usize;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(rng.random_range(0..=u8::MAX));
        }
        HexBytes(v)
    }
}

#[derive(Default, Clone, Copy)]
struct Metrics {
    inputs: u64,
    elapsed_us: u128,
}

impl Metrics {
    fn combine(self, other: Metrics) -> Metrics {
        Metrics {
            inputs: self.inputs + other.inputs,
            elapsed_us: self.elapsed_us + other.elapsed_us,
        }
    }
}

type Outcome = (Result<(), String>, Metrics);

const ALL_PROPERTIES: &[&str] =
    &["FromHexRejectsWhitespace", "InvalidCharErrorDisplayEscaped"];

fn run_all<F: FnMut(&str) -> Outcome>(mut f: F) -> Outcome {
    let mut total = Metrics::default();
    for p in ALL_PROPERTIES {
        let (r, m) = f(p);
        total = total.combine(m);
        if let Err(e) = r {
            return (Err(e), total);
        }
    }
    (Ok(()), total)
}

// ───────────── etna tool: replays frozen witness inputs. ─────────────
fn run_etna_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_etna_property);
    }
    let t0 = Instant::now();
    let result = match property {
        "FromHexRejectsWhitespace" => {
            let case1 = property_from_hex_rejects_whitespace(
                vec![0xff, 0xaa, 0xbb, 0xcc, 0xdd, 0xee],
                0,
                0,
            );
            let case2 = property_from_hex_rejects_whitespace(
                vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0],
                2,
                1,
            );
            match (case1, case2) {
                (PropertyResult::Fail(m), _) | (_, PropertyResult::Fail(m)) => Err(m),
                _ => Ok(()),
            }
        }
        "InvalidCharErrorDisplayEscaped" => {
            // Non-whitespace control bytes the buggy formatter would print raw.
            let case1 = property_invalid_char_error_display_escaped(0x00);
            let case2 = property_invalid_char_error_display_escaped(0x1b);
            match (case1, case2) {
                (PropertyResult::Fail(m), _) | (_, PropertyResult::Fail(m)) => Err(m),
                _ => Ok(()),
            }
        }
        _ => {
            return (
                Err(format!("Unknown property: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    (result, Metrics { inputs: 1, elapsed_us })
}

// ───────────── proptest ─────────────
fn bytes_strategy() -> BoxedStrategy<Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..64).boxed()
}

fn run_proptest_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_proptest_property);
    }
    let counter = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let cfg = ProptestConfig { cases: 40_000_000, ..ProptestConfig::default() };
    let mut runner = TestRunner::new(cfg);
    let result: Result<(), String> = match property {
        "FromHexRejectsWhitespace" => {
            let c = counter.clone();
            let outcome = runner.run(
                &(bytes_strategy(), any::<u32>(), any::<u8>()),
                move |(data, ws_at, ws_kind)| {
                    c.fetch_add(1, Ordering::Relaxed);
                    match property_from_hex_rejects_whitespace(data.clone(), ws_at, ws_kind) {
                        PropertyResult::Pass | PropertyResult::Discard => Ok(()),
                        PropertyResult::Fail(_) => Err(TestCaseError::fail(format!(
                            "({:?} {} {})",
                            data, ws_at, ws_kind
                        ))),
                    }
                },
            );
            match outcome {
                Ok(()) => Ok(()),
                Err(TestError::Fail(reason, _)) => Err(reason.to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
        "InvalidCharErrorDisplayEscaped" => {
            let c = counter.clone();
            let outcome = runner.run(&any::<u8>(), move |byte| {
                c.fetch_add(1, Ordering::Relaxed);
                match property_invalid_char_error_display_escaped(byte) {
                    PropertyResult::Pass | PropertyResult::Discard => Ok(()),
                    PropertyResult::Fail(_) => {
                        Err(TestCaseError::fail(format!("(0x{:02x})", byte)))
                    }
                }
            });
            match outcome {
                Ok(()) => Ok(()),
                Err(TestError::Fail(reason, _)) => Err(reason.to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
        _ => {
            return (
                Err(format!("Unknown property for proptest: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = counter.load(Ordering::Relaxed);
    (result, Metrics { inputs, elapsed_us })
}

// ───────────── quickcheck (fork with `etna` feature) ─────────────
static QC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn qc_from_hex_rejects_whitespace(
    HexBytes(data): HexBytes,
    ws_at: u32,
    ws_kind: u8,
) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_from_hex_rejects_whitespace(data, ws_at, ws_kind) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn qc_invalid_char_error_display_escaped(byte: u8) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_invalid_char_error_display_escaped(byte) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn run_quickcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_quickcheck_property);
    }
    QC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let mut qc = QuickCheck::new().tests(40_000_000).max_tests(80_000_000);
    let result = match property {
        "FromHexRejectsWhitespace" => qc.quicktest(
            qc_from_hex_rejects_whitespace as fn(HexBytes, u32, u8) -> TestResult,
        ),
        "InvalidCharErrorDisplayEscaped" => qc.quicktest(
            qc_invalid_char_error_display_escaped as fn(u8) -> TestResult,
        ),
        _ => {
            return (
                Err(format!("Unknown property for quickcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = QC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        ResultStatus::Finished => Ok(()),
        ResultStatus::Failed { arguments } => Err(format!("({})", arguments.join(" "))),
        ResultStatus::Aborted { err } => Err(format!("quickcheck aborted: {err:?}")),
        ResultStatus::TimedOut => Err("quickcheck timed out".into()),
        ResultStatus::GaveUp => Err(format!(
            "quickcheck gave up: passed={}, discarded={}",
            result.n_tests_passed, result.n_tests_discarded
        )),
    };
    // Suppress unused warnings about Gen in some builds.
    let _ = std::marker::PhantomData::<Gen>;
    (status, metrics)
}

// ───────────── crabcheck ─────────────
static CC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn cc_from_hex_rejects_whitespace(
    (HexBytes(data), ws_at, ws_kind): (HexBytes, u32, u8),
) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_from_hex_rejects_whitespace(data, ws_at, ws_kind) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

// Crabcheck requires tuple arities ≥ 2. The second element is unused; it's
// present solely to satisfy the `Arbitrary<R> for (T1, T2)` bound. Random
// variation in the ignored slot does not affect the property.
fn cc_invalid_char_error_display_escaped((byte, _ignored): (u8, u8)) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_invalid_char_error_display_escaped(byte) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn run_crabcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_crabcheck_property);
    }
    CC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let result = match property {
        "FromHexRejectsWhitespace" => crabcheck_qc::quickcheck(cc_from_hex_rejects_whitespace),
        "InvalidCharErrorDisplayEscaped" => {
            crabcheck_qc::quickcheck(cc_invalid_char_error_display_escaped)
        }
        _ => {
            return (
                Err(format!("Unknown property for crabcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = CC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        crabcheck_qc::ResultStatus::Finished => Ok(()),
        crabcheck_qc::ResultStatus::Failed { arguments } => {
            Err(format!("({})", arguments.join(" ")))
        }
        crabcheck_qc::ResultStatus::TimedOut => Err("crabcheck timed out".into()),
        crabcheck_qc::ResultStatus::GaveUp => Err(format!(
            "crabcheck gave up: passed={}, discarded={}",
            result.passed, result.discarded
        )),
        crabcheck_qc::ResultStatus::Aborted { error } => {
            Err(format!("crabcheck aborted: {error}"))
        }
    };
    (status, metrics)
}

// ───────────── hegel (hegeltest 0.3.7) ─────────────
static HG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hegel_settings() -> HegelSettings {
    HegelSettings::new().test_cases(40_000_000)
}

fn run_hegel_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_hegel_property);
    }
    HG_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let settings = hegel_settings();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match property {
        "FromHexRejectsWhitespace" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let data: Vec<u8> = tc.draw(hgen::vecs(hgen::integers::<u8>()).max_size(63));
                let ws_at: u32 = tc.draw(hgen::integers::<u32>());
                let ws_kind: u8 = tc.draw(hgen::integers::<u8>());
                if let PropertyResult::Fail(_) =
                    property_from_hex_rejects_whitespace(data.clone(), ws_at, ws_kind)
                {
                    panic!("({:?} {} {})", data, ws_at, ws_kind);
                }
            })
            .settings(settings.clone())
            .run();
        }
        "InvalidCharErrorDisplayEscaped" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let byte: u8 = tc.draw(hgen::integers::<u8>());
                if let PropertyResult::Fail(_) =
                    property_invalid_char_error_display_escaped(byte)
                {
                    panic!("(0x{:02x})", byte);
                }
            })
            .settings(settings.clone())
            .run();
        }
        _ => panic!("__unknown_property:{}", property),
    }));
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = HG_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match run_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "hegel panicked with non-string payload".to_string()
            };
            if let Some(rest) = msg.strip_prefix("__unknown_property:") {
                return (
                    Err(format!("Unknown property for hegel: {rest}")),
                    Metrics::default(),
                );
            }
            let msg = msg.strip_prefix("Property test failed: ").unwrap_or(&msg).to_string();
            Err(msg)
        }
    };
    (status, metrics)
}

fn run(tool: &str, property: &str) -> Outcome {
    match tool {
        "etna" => run_etna_property(property),
        "proptest" => run_proptest_property(property),
        "quickcheck" => run_quickcheck_property(property),
        "crabcheck" => run_crabcheck_property(property),
        "hegel" => run_hegel_property(property),
        _ => (Err(format!("Unknown tool: {tool}")), Metrics::default()),
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_json(
    tool: &str,
    property: &str,
    status: &str,
    metrics: Metrics,
    counterexample: Option<&str>,
    error: Option<&str>,
) {
    let cex = counterexample.map_or("null".to_string(), json_str);
    let err = error.map_or("null".to_string(), json_str);
    println!(
        "{{\"status\":{},\"tests\":{},\"discards\":0,\"time\":{},\"counterexample\":{},\"error\":{},\"tool\":{},\"property\":{}}}",
        json_str(status),
        metrics.inputs,
        json_str(&format!("{}us", metrics.elapsed_us)),
        cex,
        err,
        json_str(tool),
        json_str(property),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property>", args[0]);
        eprintln!("Tools: etna | proptest | quickcheck | crabcheck | hegel");
        eprintln!("Properties: FromHexRejectsWhitespace | InvalidCharErrorDisplayEscaped | All");
        std::process::exit(2);
    }
    let (tool, property) = (args[1].as_str(), args[2].as_str());

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(tool, property)));
    std::panic::set_hook(previous_hook);

    let (result, metrics) = match caught {
        Ok(outcome) => outcome,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            emit_json(
                tool,
                property,
                "aborted",
                Metrics::default(),
                None,
                Some(&format!("adapter panic: {msg}")),
            );
            return;
        }
    };

    match result {
        Ok(()) => emit_json(tool, property, "passed", metrics, None, None),
        Err(msg) => emit_json(tool, property, "failed", metrics, Some(&msg), None),
    }
}
