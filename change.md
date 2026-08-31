# Change: Cleanroom rebuild of hexyl

- **id:** rebuild-hexyl
- **title:** Rebuild hexyl 0.16.0 hex viewer from executable observation
- **why:** Trial workspace provides only the gold binary and docs; source must be recreated without referencing upstream code.
- **what:** Implement a Rust CLI hex viewer matching observed behavior (layout, CLI flags, colors, squeezing, borders, bases, include mode) and add `compile.sh` producing `./executable`.
- **out of scope:** Man page generation, packaging, upstream CI, fetching sharkdp/hexyl source.
