# hex — Injected Bugs

Total mutations: 2

## Bug Index

| # | Name | Variant | File | Injection | Fix Commit |
|---|------|---------|------|-----------|------------|
| 1 | `from_hex_accepts_whitespace` | `from_hex_accepts_whitespace_71c83f2_1` | `src/lib.rs` (in `val()`) | `marauder` | `71c83f2c7aec37c83185d4bb6d81e33f0269b7f4` |
| 2 | `invalid_char_display_raw` | `invalid_char_display_raw_764ee61_1` | `src/error.rs` (in `impl fmt::Display for FromHexError`) | `marauder` | `764ee61536cbeb8cfbce6dba61c1b85398700bb6` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `from_hex_accepts_whitespace_71c83f2_1` | `property_from_hex_rejects_whitespace` | `witness_from_hex_rejects_whitespace_case_space_at_start`, `witness_from_hex_rejects_whitespace_case_tab_mid_string` |
| `invalid_char_display_raw_764ee61_1` | `property_invalid_char_error_display_escaped` | `witness_invalid_char_error_display_escaped_case_nul`, `witness_invalid_char_error_display_escaped_case_esc` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `property_from_hex_rejects_whitespace` | ✓ | ✓ | ✓ | ✓ |
| `property_invalid_char_error_display_escaped` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. from_hex_accepts_whitespace
- **Variant**: `from_hex_accepts_whitespace_71c83f2_1`
- **Location**: `src/lib.rs`, inside `fn val(bytes: &[u8], idx: usize)` (the per-pair nibble decoder used by `decode`, `FromHex for Vec<u8>`, and `FromHex for [u8; N]`).
- **Property**: `property_from_hex_rejects_whitespace`
- **Witness(es)**: `witness_from_hex_rejects_whitespace_case_space_at_start`, `witness_from_hex_rejects_whitespace_case_tab_mid_string`
- **Fix commit**: `71c83f2c7aec37c83185d4bb6d81e33f0269b7f4` — `Stop accepting whitespace in FromHex::from_hex`.
- **Invariant violated**: `decode` must reject any byte that is not in `[0-9a-fA-F]`. In particular, ASCII whitespace bytes (`b' '`, `b'\t'`, `b'\n'`, `b'\r'`) must produce `FromHexError::InvalidHexCharacter { c, index }` pointing at the offending byte, not be silently treated as the zero nibble.
- **How the mutation triggers**: The marauder replaces the fixed `val()` body with a pre-71c83f2 variant that, for each byte, first checks `matches!(b, b' ' | b'\t' | b'\n' | b'\r')` and returns `Some(0)` for whitespace. As a result the buggy decoder silently converts inputs like `" 0ab1"` into `[0x00, 0xab, 0x10]` instead of erroring — the property observes the `Ok(_)` return and fails.

### 2. invalid_char_display_raw
- **Variant**: `invalid_char_display_raw_764ee61_1`
- **Location**: `src/error.rs`, inside `impl fmt::Display for FromHexError` at the `InvalidHexCharacter` arm.
- **Property**: `property_invalid_char_error_display_escaped`
- **Witness(es)**: `witness_invalid_char_error_display_escaped_case_nul`, `witness_invalid_char_error_display_escaped_case_esc`
- **Fix commit**: `764ee61536cbeb8cfbce6dba61c1b85398700bb6` — `Fix Error::InvalidHexCharacter::to_string`.
- **Invariant violated**: The `Display` output for `FromHexError::InvalidHexCharacter { c, index }` must render `c` via its `{:?}` debug representation so control/whitespace bytes are escaped (e.g. `'\n'`, `'\t'`). It must NOT contain the raw character verbatim, which would produce ambiguous, multi-line, or terminal-hostile messages.
- **How the mutation triggers**: The marauder switches the format string from `"Invalid character {c:?} at position {index}"` back to `"Invalid character '{}' at position {}"` (the pre-fix form). For a control byte such as `b'\n'`, `to_string()` then produces `"Invalid character '\n' at position 0"` with an embedded raw newline — the property detects the raw byte in the rendered string and fails.
