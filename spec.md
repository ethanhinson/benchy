# Spec: hexyl cleanroom rebuild

## Target

Command-line hex viewer matching `./executable` (reports `hexyl 0.16.0`).

## Inputs

- Optional file path; otherwise stdin.
- `-s/--skip`, `-n/--length` with decimal/binary prefixes, hex (`0x`), and `block` units (`--block-size`, default 512).
- Negative skip/display-offset seeks from end of file.

## Output layout (default)

- Left offset column (8 hex digits unless disabled).
- Up to two 8-byte hex panels per row (auto from terminal width).
- Optional Unicode/ASCII border or none.
- Right character panel with default table: `⋄` null, printable as-is, `_` whitespace controls, `•` other ASCII, `×` non-ASCII.
- Identical consecutive row groups collapsed to ` *` unless `-v/--no-squeezing`.

## CLI surface

Match help from gold binary: color modes, border styles, plain preset, character/color schemes, panels, group size/endianness, numeric base, terminal width, include mode, color table, shell completions, version/help.

## Build

- Rust + Cargo project in repo root.
- `compile.sh` runs release build and writes `./executable` (must not invoke gold binary).

## Verification

Run rebuilt binary with `--color=never --border=none` on fixed test inputs; compare to saved gold reference output for core cases.
