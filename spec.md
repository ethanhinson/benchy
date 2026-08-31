# Spec: eva calculator rebuild

## Architecture

Rust CLI application with four modules:

| Module | Responsibility |
|--------|----------------|
| `main.rs` | CLI (clap), REPL loop, history persistence |
| `parser.rs` | Tokenize, implicit `*` insertion, paren balancing, AST |
| `eval.rs` | Evaluate AST, math functions, angle units, previous answer |
| `format.rs` | Output formatting by base (1–36) and decimal places (1–64) |

## CLI contract (observed)

```
Usage: executable [OPTIONS] [INPUT]
  -f, --fix <FIX>                decimal places 1–64, default 10
  -b, --base <RADIX>             output radix 1–36, default 10
  -a, --angle_unit <angle_unit>  degree | radian | gradian, default degree
  -h, --help
  -V, --version                  prints "eva 0.3.1"
```

Command mode: evaluate INPUT once, print result, exit 0 (or 1 on error).

REPL mode: read lines until EOF; prompt `> `; show `No previous history.` when history file is new/empty.

## Expression language

- Binary: `+ - * / ^ **` (right-associative power)
- Unary: `+ -`
- Constants: `pi`, `e`, `_` (previous answer)
- Functions (1 arg): sin, cos, tan, csc, sec, cot, sinh, cosh, tanh, asin, acos, atan, acsc, asec, acot, ln, log2, log10, sqrt, ceil, floor, abs
- Functions (2 arg): log, nroot
- Conversion: deg(x), rad(x)
- Implicit multiplication: `12sin(45(2))` → `12*sin(45*(2))`
- Functions require parentheses: `sin 30` → Syntax Error
- REPL auto-closes unmatched `(` before evaluation

## Errors (exact strings observed)

- `Domain Error: Out of bounds!`
- `Math Error: Divide by zero error!`
- `Parser Error: Too many operators, too few operands`
- `Syntax Error: Function 'NAME' expected parentheses`

## History

Path: `~/.local/share/eva/history.txt`, format starts with `#V2` header line.

## Output formatting

- Base 10: comma-separated integer part, `.` + fix decimal places
- Base ≠ 10: fixed-width digit groups with commas, uppercase digits A–Z, `.0` suffix

## Build

`compile.sh` runs `cargo build --release` and copies artifact to `./executable`.
