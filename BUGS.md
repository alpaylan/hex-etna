# hex — Injected Bugs

Total mutations: 2

## Bug Index

| # | Variant | Name | Location | Injection | Fix Commit |
|---|---------|------|----------|-----------|------------|
| 1 | `from_hex_accepts_whitespace_71c83f2_1` | `from_hex_accepts_whitespace` | `src/lib.rs` | `marauders` | `71c83f2349d847ec95bf1ee6080ecdc7ca665037` |
| 2 | `invalid_char_display_raw_764ee61_1` | `invalid_char_display_raw` | `src/error.rs` | `marauders` | `764ee61536cbeb8cfbce6dba61c1b85398700bb6` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `from_hex_accepts_whitespace_71c83f2_1` | `FromHexRejectsWhitespace` | `witness_from_hex_rejects_whitespace_case_space_at_start`, `witness_from_hex_rejects_whitespace_case_tab_mid_string` |
| `invalid_char_display_raw_764ee61_1` | `InvalidCharErrorDisplayEscaped` | `witness_invalid_char_error_display_escaped_case_nul`, `witness_invalid_char_error_display_escaped_case_esc` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `FromHexRejectsWhitespace` | ✓ | ✓ | ✓ | ✓ |
| `InvalidCharErrorDisplayEscaped` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. from_hex_accepts_whitespace

- **Variant**: `from_hex_accepts_whitespace_71c83f2_1`
- **Location**: `src/lib.rs`
- **Property**: `FromHexRejectsWhitespace`
- **Witness(es)**:
  - `witness_from_hex_rejects_whitespace_case_space_at_start`
  - `witness_from_hex_rejects_whitespace_case_tab_mid_string`
- **Source**: Stop accepting whitespace in `FromHex::from_hex`.
  > `FromHex::from_hex` historically accepted ASCII whitespace and silently treated each whitespace byte as the `0` nibble, quietly corrupting decoded output. The fix drops the whitespace short-circuit so any non-hex byte produces `InvalidHexCharacter`.
- **Fix commit**: `71c83f2349d847ec95bf1ee6080ecdc7ca665037` — Stop accepting whitespace in `FromHex::from_hex`.
- **Invariant violated**: `decode` must reject any byte that is not in `[0-9a-fA-F]`. In particular, ASCII whitespace bytes (`b' '`, `b'\t'`, `b'\n'`, `b'\r'`) must produce `FromHexError::InvalidHexCharacter { c, index }` pointing at the offending byte, not be silently treated as the zero nibble.
- **How the mutation triggers**: The marauder replaces the fixed `val()` body with a pre-71c83f2 variant that, for each byte, first checks `matches!(b, b' ' | b'\t' | b'\n' | b'\r')` and returns `Some(0)` for whitespace. As a result the buggy decoder silently converts inputs like `" 0ab1"` into `[0x00, 0xab, 0x10]` instead of erroring — the property observes the `Ok(_)` return and fails.

### 2. invalid_char_display_raw

- **Variant**: `invalid_char_display_raw_764ee61_1`
- **Location**: `src/error.rs`
- **Property**: `InvalidCharErrorDisplayEscaped`
- **Witness(es)**:
  - `witness_invalid_char_error_display_escaped_case_nul`
  - `witness_invalid_char_error_display_escaped_case_esc`
- **Source**: Fix `Error::InvalidHexCharacter::to_string`.
  > The pre-fix `Display` impl formatted `InvalidHexCharacter` with `'{}'` around the raw byte, so control characters (newline, tab, NUL, escape) appeared literally in error messages and broke terminal output. The fix uses the `{:?}` debug formatter to escape non-printable bytes.
- **Fix commit**: `764ee61536cbeb8cfbce6dba61c1b85398700bb6` — Fix `Error::InvalidHexCharacter::to_string`.
- **Invariant violated**: The `Display` output for `FromHexError::InvalidHexCharacter { c, index }` must render `c` via its `{:?}` debug representation so control/whitespace bytes are escaped (e.g. `'\n'`, `'\t'`). It must NOT contain the raw character verbatim, which would produce ambiguous, multi-line, or terminal-hostile messages.
- **How the mutation triggers**: The marauder switches the format string from `"Invalid character {c:?} at position {index}"` back to `"Invalid character '{}' at position {}"` (the pre-fix form). For a control byte such as `b'\n'`, `to_string()` then produces `"Invalid character '\n' at position 0"` with an embedded raw newline — the property detects the raw byte in the rendered string and fails.
