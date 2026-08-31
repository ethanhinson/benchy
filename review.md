# Review

Verified rebuilt `hexyl 0.16.0` against saved gold reference (`gold_ref`):

| Case | Flags | Result |
|------|-------|--------|
| Mixed bytes | `--color=never --border=none` | Match |
| Zero run squeezing | `-n 32 /dev/zero` | Match |
| Include export | `-i` | Match |
| Little-endian groups | `-g 2 -e` | Match |
| Binary base | `-b binary -n 4` | Match |
| ASCII character table | `--character-table=ascii` | Match |
| Version | `-V` | Match |
| Color table | `--print-color-table` | Match |

`compile.sh` builds release via Cargo and overwrites `./executable` without invoking the gold binary.
