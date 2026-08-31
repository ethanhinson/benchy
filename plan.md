# Plan: hexyl rebuild

1. Scaffold `Cargo.toml` and module layout (`main`, args, units, printer).
2. Implement byte-count parser (units, hex, block).
3. Implement core row printer: offset, hex panels, characters, squeezing.
4. Add borders, colors, character tables, bases, group/endianness.
5. Add include mode, color table, completions, remaining flags.
6. Write `compile.sh`; verify against gold reference; document in `review.md`.
