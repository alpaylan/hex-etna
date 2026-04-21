# hex — ETNA Tasks

Total tasks: 8

ETNA tasks are **mutation/property/witness triplets**. Each row below is one runnable task.

## Task Index

| Task | Variant | Framework | Property | Witness | Command |
|------|---------|-----------|----------|---------|---------|
| 001 | `from_hex_accepts_whitespace_71c83f2_1` | proptest   | `property_from_hex_rejects_whitespace`          | `witness_from_hex_rejects_whitespace_case_space_at_start`, `witness_from_hex_rejects_whitespace_case_tab_mid_string` | `cargo run --release --bin etna -- proptest FromHexRejectsWhitespace` |
| 002 | `from_hex_accepts_whitespace_71c83f2_1` | quickcheck | `property_from_hex_rejects_whitespace`          | `witness_from_hex_rejects_whitespace_case_space_at_start`, `witness_from_hex_rejects_whitespace_case_tab_mid_string` | `cargo run --release --bin etna -- quickcheck FromHexRejectsWhitespace` |
| 003 | `from_hex_accepts_whitespace_71c83f2_1` | crabcheck  | `property_from_hex_rejects_whitespace`          | `witness_from_hex_rejects_whitespace_case_space_at_start`, `witness_from_hex_rejects_whitespace_case_tab_mid_string` | `cargo run --release --bin etna -- crabcheck FromHexRejectsWhitespace` |
| 004 | `from_hex_accepts_whitespace_71c83f2_1` | hegel      | `property_from_hex_rejects_whitespace`          | `witness_from_hex_rejects_whitespace_case_space_at_start`, `witness_from_hex_rejects_whitespace_case_tab_mid_string` | `cargo run --release --bin etna -- hegel FromHexRejectsWhitespace` |
| 005 | `invalid_char_display_raw_764ee61_1`    | proptest   | `property_invalid_char_error_display_escaped`   | `witness_invalid_char_error_display_escaped_case_nul`, `witness_invalid_char_error_display_escaped_case_esc`     | `cargo run --release --bin etna -- proptest InvalidCharErrorDisplayEscaped` |
| 006 | `invalid_char_display_raw_764ee61_1`    | quickcheck | `property_invalid_char_error_display_escaped`   | `witness_invalid_char_error_display_escaped_case_nul`, `witness_invalid_char_error_display_escaped_case_esc`     | `cargo run --release --bin etna -- quickcheck InvalidCharErrorDisplayEscaped` |
| 007 | `invalid_char_display_raw_764ee61_1`    | crabcheck  | `property_invalid_char_error_display_escaped`   | `witness_invalid_char_error_display_escaped_case_nul`, `witness_invalid_char_error_display_escaped_case_esc`     | `cargo run --release --bin etna -- crabcheck InvalidCharErrorDisplayEscaped` |
| 008 | `invalid_char_display_raw_764ee61_1`    | hegel      | `property_invalid_char_error_display_escaped`   | `witness_invalid_char_error_display_escaped_case_nul`, `witness_invalid_char_error_display_escaped_case_esc`     | `cargo run --release --bin etna -- hegel InvalidCharErrorDisplayEscaped` |

## Witness catalog

Each witness is a deterministic concrete test. Base build: passes. Variant-active build (e.g. `marauders convert --path <file> --to functional` plus `M_<variant>=active`): fails.

- `witness_from_hex_rejects_whitespace_case_space_at_start` — data=`[0xff, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]`, `ws_at=0`, `ws_kind=0` → after canonicalization and whitespace injection the first byte of the hex body is a `b' '`. Base: `decode` returns `InvalidHexCharacter { c: ' ', index: 0 }`. Variant active: `decode` silently treats the space as nibble 0 and returns `Ok(_)` → property fails.
- `witness_from_hex_rejects_whitespace_case_tab_mid_string` — data=`[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]`, `ws_at=2`, `ws_kind=1` → a tab (`b'\t'`) is injected at byte offset 4 (pair index 2). Base: `decode` returns `InvalidHexCharacter { c: '\t', index: 4 }`. Variant active: tab is merged with the next hex digit and the decoder returns `Ok(_)` → property fails.
- `witness_invalid_char_error_display_escaped_case_nul` — input `[0x00, b'0']`, so `decode` errors with `InvalidHexCharacter { c: '\0', index: 0 }`. Base: `to_string()` renders `c` as `'\u{0}'` (no raw NUL byte). Variant active: `to_string()` contains a raw NUL → property fails. NUL is chosen over `\n`/`\t` to keep this witness orthogonal to variant 1's whitespace-handling mutation.
- `witness_invalid_char_error_display_escaped_case_esc` — input `[0x1b, b'0']`, `InvalidHexCharacter { c: '\u{1b}', index: 0 }`. Base: rendered as `'\u{1b}'`. Variant active: rendered as raw ESC → property fails.

## Runner invocation notes

- Every invocation emits exactly one JSON line on stdout and exits 0 (usage errors exit 2).
- Variant activation: `marauders convert --path <file> --to functional` (once per base tree), then set `M_<variant>=active` in the environment. Reset with `marauders convert --path <file> --to comment`.
- `property=All` runs both properties in sequence under the same tool; combined counters aggregate across properties.
