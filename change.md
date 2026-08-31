# Change: Cleanroom rebuild of tuc 1.3.0

## Id
docket-superpowers-slice-c-1

## Title
Rebuild `tuc` text-cutting CLI from executable observation and docs

## Why
Trial workspace provides only the gold `./executable` and `docs/README.md`. Source must be recreated without decompilation or copying the binary.

## What
- Original Rust implementation matching documented CLI and observed behavior
- `compile.sh` producing `./executable` via `cargo build --release`
- Docket lifecycle artifacts (spec, plan, review)

## Out of scope
- Benchmark suite, playground, packaging
- `--no-mmap`, `--fixed-memory` streaming optimizations (stub or minimal)
- Internet lookup of upstream source
