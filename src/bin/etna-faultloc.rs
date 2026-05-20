// Crabcheck fault-localization runner for hex.
//
// Mirrors the aho-corasick / crc32fast / ryu etna-faultloc binaries.
// Self-contained — the existing `etna` runner is untouched.

use std::fmt;

use crabcheck::profiling::quickcheck;
use crabcheck::quickcheck::{Arbitrary, Mutate};
use hex::etna::{
    property_from_hex_rejects_whitespace, property_invalid_char_error_display_escaped,
    PropertyResult,
};
use rand::Rng;

// ---------- Property 1: FromHexRejectsWhitespace ----------
//
// Inputs: Vec<u8> (data lifted into hex alphabet inside the property),
// u32 (which even-pair index gets the whitespace), u8 (which whitespace
// char to use, mod 4). Wrap them in one struct so Arbitrary+Mutate can
// perturb one field at a time, BST-style.

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
        // Pick one field, perturb a single byte/bit inside it.
        match rng.random_range(0u8..3) {
            0 => {
                if !out.data.is_empty() {
                    let i = rng.random_range(0..out.data.len());
                    out.data[i] = rng.random::<u8>();
                } else {
                    out.data.push(rng.random::<u8>());
                }
            },
            1 => {
                let bit = rng.random_range(0u32..32);
                out.ws_at ^= 1u32 << bit;
            },
            _ => {
                let bit = rng.random_range(0u32..8);
                out.ws_kind ^= 1u8 << bit;
            },
        }
        out
    }
}

// ---------- Property 2: InvalidCharErrorDisplayEscaped ----------
//
// Input: a single u8 byte. The property discards anything that's a valid
// hex char, so most random bytes get tested. Mutate flips one bit.

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

// ---------- Helpers ----------

fn to_opt(r: PropertyResult) -> Option<bool> {
    match r {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}


// ---------- Dispatcher ----------

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property> [tests]", args[0]);
        eprintln!("  tool:     crabcheck");
        eprintln!("  property: FromHexRejectsWhitespace | InvalidCharErrorDisplayEscaped");
        return;
    }
    let tool = args[1].as_str();
    let property = args[2].as_str();

    let result = match (tool, property) {
        ("crabcheck", "FromHexRejectsWhitespace") => {
            quickcheck(|w: WhitespaceInput| {
                {
                    to_opt(property_from_hex_rejects_whitespace(
                        w.data, w.ws_at, w.ws_kind,
                    ))
                }
            })
        },
        ("crabcheck", "InvalidCharErrorDisplayEscaped") => {
            quickcheck(|ByteInput(b)| {
                to_opt(property_invalid_char_error_display_escaped(b))
            })
        },
        _ => panic!("Unknown tool or property: {tool} {property}"),
    };

    println!("Result: {:?}", result);
}
