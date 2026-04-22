# hex — ETNA Tasks

Total tasks: 8

## Task Index

| Task | Variant | Framework | Property | Witness |
|------|---------|-----------|----------|---------|
| 001 | `from_hex_accepts_whitespace_71c83f2_1` | proptest | `FromHexRejectsWhitespace` | `witness_from_hex_rejects_whitespace_case_space_at_start` |
| 002 | `from_hex_accepts_whitespace_71c83f2_1` | quickcheck | `FromHexRejectsWhitespace` | `witness_from_hex_rejects_whitespace_case_space_at_start` |
| 003 | `from_hex_accepts_whitespace_71c83f2_1` | crabcheck | `FromHexRejectsWhitespace` | `witness_from_hex_rejects_whitespace_case_space_at_start` |
| 004 | `from_hex_accepts_whitespace_71c83f2_1` | hegel | `FromHexRejectsWhitespace` | `witness_from_hex_rejects_whitespace_case_space_at_start` |
| 005 | `invalid_char_display_raw_764ee61_1` | proptest | `InvalidCharErrorDisplayEscaped` | `witness_invalid_char_error_display_escaped_case_nul` |
| 006 | `invalid_char_display_raw_764ee61_1` | quickcheck | `InvalidCharErrorDisplayEscaped` | `witness_invalid_char_error_display_escaped_case_nul` |
| 007 | `invalid_char_display_raw_764ee61_1` | crabcheck | `InvalidCharErrorDisplayEscaped` | `witness_invalid_char_error_display_escaped_case_nul` |
| 008 | `invalid_char_display_raw_764ee61_1` | hegel | `InvalidCharErrorDisplayEscaped` | `witness_invalid_char_error_display_escaped_case_nul` |

## Witness Catalog

- `witness_from_hex_rejects_whitespace_case_space_at_start` — base passes, variant fails
- `witness_from_hex_rejects_whitespace_case_tab_mid_string` — base passes, variant fails
- `witness_invalid_char_error_display_escaped_case_nul` — base passes, variant fails
- `witness_invalid_char_error_display_escaped_case_esc` — base passes, variant fails
