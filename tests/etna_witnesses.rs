//! Deterministic witness tests for hex ETNA variants.
//!
//! Each `witness_<name>_case_<tag>` passes on the base HEAD and fails under
//! the corresponding `etna/<variant>` branch (or with the matching mutation
//! activated). Witnesses call `property_<name>` directly with frozen inputs —
//! no proptest/quickcheck/RNG/clock machinery.

#![cfg(feature = "etna")]

use hex::etna::{
    property_from_hex_rejects_whitespace, property_invalid_char_error_display_escaped,
    PropertyResult,
};

fn expect_pass(r: PropertyResult, what: &str) {
    match r {
        PropertyResult::Pass => {}
        PropertyResult::Fail(m) => panic!("{}: property failed: {}", what, m),
        PropertyResult::Discard => panic!("{}: unexpected discard", what),
    }
}

// ────────── Variant: from_hex_accepts_whitespace_71c83f2_1 ──────────

// Buggy code silently treats b' ' as nibble 0, so input " 0ab1" decodes to
// [0x00, 0xab, 0x10] instead of erroring. Fixed code rejects with
// InvalidHexCharacter { c: ' ', index: 0 }.
#[test]
fn witness_from_hex_rejects_whitespace_case_space_at_start() {
    // Body produced by the property from data=[0xff, 0x0a, 0xb1, 0xc2, 0xd3]
    // is "f0abc2d3"-ish; what matters is that ws_at=0 picks pair 0 and
    // ws_kind=0 picks b' '. The property masks bytes into [0-9a-f] and
    // overwrites index 0 with a space.
    expect_pass(
        property_from_hex_rejects_whitespace(vec![0xff, 0xaa, 0xbb, 0xcc, 0xdd, 0xee], 0, 0),
        "from_hex_rejects_whitespace / space_at_start",
    );
}

// A mid-string tab: six hex bytes, tab injected at pair offset 2 (byte
// index 4). Buggy code treats the tab as nibble 0 and merges it with the
// next hex digit; fixed code raises InvalidHexCharacter at index 4.
#[test]
fn witness_from_hex_rejects_whitespace_case_tab_mid_string() {
    expect_pass(
        property_from_hex_rejects_whitespace(
            vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0],
            2,
            1,
        ),
        "from_hex_rejects_whitespace / tab_mid_string",
    );
}

// ────────── Variant: invalid_char_display_raw_764ee61_1 ──────────

// Buggy display format '{}' interpolates NUL as a literal 0x00 byte inside
// the error message; fixed format {:?} escapes it as '\u{0}'. The property
// inspects `to_string()` of the resulting FromHexError for the presence of
// the raw character. NUL is chosen over `\n`/`\t` so this witness isolates
// the display bug from the whitespace-handling bug in variant 1.
#[test]
fn witness_invalid_char_error_display_escaped_case_nul() {
    expect_pass(
        property_invalid_char_error_display_escaped(0x00),
        "invalid_char_error_display_escaped / nul",
    );
}

// ESC (0x1b): another control byte that's neither hex-valid nor
// whitespace. Buggy formatter leaks raw ESC; fixed formatter escapes.
#[test]
fn witness_invalid_char_error_display_escaped_case_esc() {
    expect_pass(
        property_invalid_char_error_display_escaped(0x1b),
        "invalid_char_error_display_escaped / esc",
    );
}
