//! ETNA framework-neutral property functions for `hex`.
//!
//! Each `property_<name>` is a pure function taking concrete, owned inputs and
//! returning `PropertyResult`. Framework adapters (proptest/quickcheck/crabcheck/hegel)
//! in `src/bin/etna.rs` and deterministic witness tests in `tests/etna_witnesses.rs`
//! both call these functions directly — no re-implementation of the invariant
//! inside any adapter.

#![allow(missing_docs)]

use crate::{decode, FromHexError};

pub enum PropertyResult {
    Pass,
    Fail(String),
    Discard,
}

// ──────────────────────────────────────────────────────────────────────────
// Property: `decode` rejects any input that contains an ASCII whitespace
// character at an even-indexed position (i.e., inside a 2-char hex pair),
// returning `InvalidHexCharacter` pointing at the whitespace byte.
//
// Regression for fix commit 71c83f2 (Mar 2016) — "Stop accepting whitespace
// in FromHex::from_hex". Prior versions silently treated bytes like b' ',
// b'\t', b'\n', b'\r' as the zero nibble, so inputs such as "1 2 " would
// decode to [0x10, 0x20] rather than erroring.
//
// The property takes an arbitrary byte slice, clamps it to a canonical hex
// body (using a bounded-hex alphabet), then overwrites a selected even-offset
// position with one of the four whitespace characters. The resulting string
// MUST decode to `Err(InvalidHexCharacter { .. })` with the index equal to
// the clobbered offset and the character being the whitespace we wrote.
// ──────────────────────────────────────────────────────────────────────────
pub fn property_from_hex_rejects_whitespace(
    data: Vec<u8>,
    ws_at: u32,
    ws_kind: u8,
) -> PropertyResult {
    // Need at least one full pair to have any hex body at all.
    if data.is_empty() {
        return PropertyResult::Discard;
    }

    // Canonicalise the body: even length, each byte in [0-9a-f]. We lift
    // arbitrary bytes into valid hex chars by masking with the 16-char
    // alphabet so that, absent the injected whitespace, decoding would
    // succeed.
    const ALPHABET: &[u8; 16] = b"0123456789abcdef";
    let mut body: Vec<u8> = data.iter().map(|b| ALPHABET[(*b as usize) & 0x0f]).collect();
    if body.len() % 2 != 0 {
        body.push(b'0');
    }
    // Bound size to keep the test domain small.
    if body.len() > 256 {
        body.truncate(256);
    }
    if body.is_empty() {
        return PropertyResult::Discard;
    }

    // Pick an even offset at which to inject whitespace. Using an even
    // offset matters because the decoder reads byte pairs two at a time;
    // we want the whitespace to be the *upper* nibble of a pair so the
    // buggy pre-71c83f2 code would merge it with the next valid char.
    let pair_count = (body.len() / 2) as u32;
    let pair_idx = (ws_at % pair_count) as usize;
    let idx = pair_idx * 2;

    let ws = match ws_kind % 4 {
        0 => b' ',
        1 => b'\t',
        2 => b'\n',
        _ => b'\r',
    };
    body[idx] = ws;

    match decode(&body) {
        Err(FromHexError::InvalidHexCharacter { c, index }) => {
            if c as u32 != ws as u32 {
                return PropertyResult::Fail(format!(
                    "decode returned InvalidHexCharacter for {c:?} but injected byte was {:?}",
                    ws as char
                ));
            }
            if index != idx {
                return PropertyResult::Fail(format!(
                    "decode reported index {index} but whitespace was at {idx}"
                ));
            }
            PropertyResult::Pass
        }
        Err(other) => PropertyResult::Fail(format!(
            "decode returned {other:?} instead of InvalidHexCharacter; injected {:?} at {idx}",
            ws as char
        )),
        Ok(out) => PropertyResult::Fail(format!(
            "decode silently accepted whitespace at {idx}: got {out:?}"
        )),
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Property: the `Display` impl for `FromHexError::InvalidHexCharacter` never
// renders a raw control/whitespace character into the message — it escapes
// them via the `{:?}` formatter.
//
// Regression for fix commit 764ee61 (Dec 2019) — "Fix
// Error::InvalidHexCharacter::to_string". Prior versions used
// `'{}'`-style interpolation which emitted the raw char, so a control byte
// like 0x0a would produce a literal newline in the middle of the error
// message. Post-fix, the same byte renders as `'\n'`.
//
// The property builds a valid-length (2-byte) hex input whose first byte is
// an unprintable byte (control char or non-ASCII), asks `decode` for the
// error, and asserts the error's `Display` output does NOT contain the raw
// problem byte as a character. Under the buggy formatting, a newline in the
// input becomes a newline in `to_string()`, which this check catches.
// ──────────────────────────────────────────────────────────────────────────
pub fn property_invalid_char_error_display_escaped(byte: u8) -> PropertyResult {
    // Only test bytes that are actually invalid hex AND would render unsafely
    // if interpolated raw (i.e. ASCII control / whitespace / non-printables).
    let is_hex = matches!(byte, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F');
    if is_hex {
        return PropertyResult::Discard;
    }
    // Restrict to bytes whose raw rendering would be observably unsafe —
    // i.e. ASCII control characters. These are the bytes the pre-764ee61
    // formatter would have printed literally.
    if !(byte < 0x20 || byte == 0x7f) {
        return PropertyResult::Discard;
    }

    // Build a 2-char input where the first byte is our problem byte and
    // the second is a valid hex digit. `decode` must produce
    // InvalidHexCharacter { c: byte as char, index: 0 }.
    let input = [byte, b'0'];
    let err = match decode(&input[..]) {
        Err(e) => e,
        Ok(_) => {
            return PropertyResult::Fail(format!(
                "decode unexpectedly succeeded for invalid byte 0x{byte:02x}"
            ))
        }
    };
    // Sanity-check the error shape before inspecting Display.
    match err {
        FromHexError::InvalidHexCharacter { index, .. } => {
            if index != 0 {
                return PropertyResult::Fail(format!(
                    "expected InvalidHexCharacter at index 0, got index {index}"
                ));
            }
        }
        other => {
            return PropertyResult::Fail(format!(
                "expected InvalidHexCharacter, got {other:?}"
            ));
        }
    }

    let rendered = err.to_string();
    // Under the buggy formatter the raw byte appears verbatim. A correctly
    // debug-escaped `{c:?}` rendering will never contain the raw control
    // character, because chars < 0x20 and 0x7f are always escaped.
    if rendered.chars().any(|c| c == byte as char) {
        return PropertyResult::Fail(format!(
            "error Display leaked raw byte 0x{byte:02x}: {:?}",
            rendered
        ));
    }
    PropertyResult::Pass
}
