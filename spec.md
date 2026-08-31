# Spec: tuc cleanroom rebuild

## Architecture
- **Language:** Rust (2021 edition), `clap` for CLI, `regex` for `-e`
- **Layout:** `src/main.rs` (entry + line loop), `src/cli.rs` (args), `src/bounds.rs` (field specs), `src/format.rs` (template `{n}` / `{a:b}`), `src/split.rs` (delimiter split/greedy/compress/trim), `src/process.rs` (fields/bytes/chars/lines modes)
- **Output binary:** `target/release/tuc` copied to `./executable` by `compile.sh`

## Prompt contract (from docs + gold runs)
| Feature | Behavior |
|---------|----------|
| Default | `-d '\t'`, `-f 1:` |
| Fields | 1-indexed, ranges `:`, negatives, `=fallback`, reorder with commas |
| Ranges | Join with delimiter; lists without `:` concatenate unless `-j` |
| `-j` | Join selected parts with delimiter |
| `-r` | Implies join; replace delimiter between parts |
| `-l` | Line mode; implies join with `\n`; `--no-join` merges |
| `-m` | Complement selected bounds |
| `-g` / `-p` | Greedy / compress delimiters before split |
| `-s` | Skip lines without delimiter (unless whole line is one field?) |
| `-e` | Regex delimiter |
| `--json` | JSON array of selected field strings |
| Format `-f '{…}'` | `{n}`, `{a:b}`, `\n`, `{{` `}}` escape |
| Errors | `Error: Out of bounds: N` on stderr, exit 1 |
| No args | Banner + examples to stdout |
| `--version` | `tuc 1.3.0` |

## Verification
Compare rebuilt `./executable` against saved gold runs for core cases (fields, join, json, regex, complement, bytes/chars, format, errors).
