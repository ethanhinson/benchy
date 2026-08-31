# Review

Verified rebuilt `./executable` against gold binary restored from git (`HEAD:executable`).

## Checks run
| Area | Result |
|------|--------|
| Default banner (no args) | Match |
| `--version`, `--help` head | Match |
| Field cut/join/complement | Match |
| Ranges, negatives, reorder | Match |
| `-g`, `-p`, `-r`, `-t` | Match |
| Regex delimiter `-e` | Match |
| Format strings `{n}`, `{a:}`, `\n` | Match |
| `--json` | Match |
| `-b` / `-c` UTF-8 slices | Match |
| `-l` lines (incl. `--no-join`, `-1` → `1:`) | Match |
| OOB errors and fallbacks | Match |
| File + stdin input | Match |

## Notes
- Gold aborts on `-f '{{1}}'` (SIGABRT); rebuild returns literal `{1}` per docs escape rules.
- Gold `-l 2:-2` errors; rebuild matches that error path.

## Build
`./compile.sh` → `cargo build --release` → copies `target/release/tuc` to `./executable`.
