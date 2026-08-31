# Review: eva cleanroom rebuild

## Observed behaviors verified

| Area | Checks |
|------|--------|
| CLI | `--version` → `eva 0.3.1`; `--help` text matches gold |
| Command mode | `1 + sin(30)`, `floor(sqrt(3^2 + 5^2))`, implicit multiply, power ops |
| Errors | Domain, divide-by-zero, parser, syntax (missing parens) messages match |
| Flags | `-f` decimal places, `-b` radix output (including non-decimal grouping), `-a` angle units |
| Functions | Trig (degree default), log, nroot, sqrt, deg/rad, constants pi/e/_ |
| REPL | `No previous history.` on empty history; `> ` prompt; piped multi-line input |
| Parens | Auto-balance unmatched `(` in REPL |
| History | Persists to `~/.local/share/eva/history.txt` with `#V2` header |

## Build

`./compile.sh` runs `cargo build --release` and copies `target/release/eva` to `./executable`.

## Constraints followed

- No decompilation, disassembly, or gold-binary wrapping
- Source written from `docs/readme.md` and runtime observation only
- Docket lifecycle: change.md, spec.md, plan.md, implementation, review.md
