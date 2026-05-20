//! Fault-localization integration tests for hex.
//!
//! One `#[test]` per property in src/bin/etna-faultloc.rs's dispatch.
//! Each test runs `crabcheck::quickcheck_with_locate!` on the property,
//! prints the report, and emits a single `@@LOCATE@@ {<json>}` line on
//! stdout. Tests never panic — the driver classifies success/failure
//! from the JSON.

#![cfg(feature = "etna")]

use std::fmt;

use crabcheck::quickcheck::{Arbitrary, Mutate};
use hex::etna::{
    property_from_hex_rejects_whitespace, property_invalid_char_error_display_escaped,
    PropertyResult,
};
use rand::Rng;

#[derive(Clone)]
struct WhitespaceInput {
    data: Vec<u8>,
    ws_at: u32,
    ws_kind: u8,
}

impl fmt::Debug for WhitespaceInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "data={:?} ws_at={} ws_kind={}",
            self.data, self.ws_at, self.ws_kind
        )
    }
}

const MAX_DATA_LEN: usize = 64;

impl<R: Rng> Arbitrary<R> for WhitespaceInput {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let len = rng.random_range(1usize..=MAX_DATA_LEN);
        let data: Vec<u8> = (0..len).map(|_| rng.random::<u8>()).collect();
        WhitespaceInput {
            data,
            ws_at: rng.random::<u32>(),
            ws_kind: rng.random::<u8>(),
        }
    }
}

impl<R: Rng> Mutate<R> for WhitespaceInput {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = self.clone();
        match rng.random_range(0u8..3) {
            0 => {
                if !out.data.is_empty() {
                    let i = rng.random_range(0..out.data.len());
                    out.data[i] = rng.random::<u8>();
                } else {
                    out.data.push(rng.random::<u8>());
                }
            }
            1 => {
                let bit = rng.random_range(0u32..32);
                out.ws_at ^= 1u32 << bit;
            }
            _ => {
                let bit = rng.random_range(0u32..8);
                out.ws_kind ^= 1u8 << bit;
            }
        }
        out
    }
}

#[derive(Clone, Copy)]
struct ByteInput(u8);

impl fmt::Debug for ByteInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}", self.0)
    }
}

impl<R: Rng> Arbitrary<R> for ByteInput {
    fn generate(rng: &mut R, _n: usize) -> Self {
        ByteInput(rng.random::<u8>())
    }
}

impl<R: Rng> Mutate<R> for ByteInput {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let bit = rng.random_range(0u32..8);
        ByteInput(self.0 ^ (1u8 << bit))
    }
}

fn to_opt(r: PropertyResult) -> Option<bool> {
    match r {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn property_from_hex_rejects_whitespace_test(w: WhitespaceInput) -> Option<bool> {
    to_opt(property_from_hex_rejects_whitespace(
        w.data, w.ws_at, w.ws_kind,
    ))
}

fn property_invalid_char_error_display_escaped_test(ByteInput(b): ByteInput) -> Option<bool> {
    to_opt(property_invalid_char_error_display_escaped(b))
}

fn emit_locate_json(r: &crabcheck::profiling::LocateResult) {
    use crabcheck::quickcheck::ResultStatus;
    let status = match &r.run.status {
        ResultStatus::Failed { .. } => "Failed",
        ResultStatus::Finished => "Finished",
        ResultStatus::GaveUp => "GaveUp",
        ResultStatus::TimedOut => "TimedOut",
        ResultStatus::Aborted { .. } => "Aborted",
    };
    let top = if let Some(s) = r.top() {
        serde_json::json!({
            "rank": s.rank,
            "file": s.region.file,
            "function": s.region.function,
            "start_line": s.region.start_line,
            "end_line": s.region.end_line,
            "ochiai": s.region.suspiciousness.ochiai,
            "delta": s.region.delta,
            "panic_overlap": s.panic_overlap,
            "confidence": format!("{}", s.confidence),
            "confidence_rule": s.confidence_rule,
        })
    } else {
        serde_json::Value::Null
    };
    let top_5: Vec<_> = r
        .suspects
        .iter()
        .take(5)
        .map(|s| {
            serde_json::json!({
                "rank": s.rank,
                "file": s.region.file,
                "function": s.region.function,
                "start_line": s.region.start_line,
                "end_line": s.region.end_line,
                "confidence": format!("{}", s.confidence),
                "confidence_rule": s.confidence_rule,
                "panic_overlap": s.panic_overlap,
            })
        })
        .collect();
    let diags: Vec<_> = r.diagnostics.iter().map(|d| d.tag()).collect();
    let out = serde_json::json!({
        "status": status,
        "passed": r.run.passed,
        "discarded": r.run.discarded,
        "n_panics": r.n_panics,
        "n_suspects": r.suspects.len(),
        "top": top,
        "top_5": top_5,
        "diagnostics": diags,
    });
    println!("@@LOCATE@@ {}", out);
}

#[test]
fn locate_from_hex_rejects_whitespace() {
    let report =
        crabcheck::quickcheck_with_locate!(property_from_hex_rejects_whitespace_test, "hex");
    eprintln!("{report}");
    emit_locate_json(&report);
}

#[test]
fn locate_invalid_char_error_display_escaped() {
    let report = crabcheck::quickcheck_with_locate!(
        property_invalid_char_error_display_escaped_test,
        "hex"
    );
    eprintln!("{report}");
    emit_locate_json(&report);
}
