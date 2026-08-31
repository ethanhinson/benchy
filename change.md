# Change: Cleanroom rebuild of eva calculator

**ID:** eva-rebuild-001

**Title:** Rebuild eva 0.3.1 calculator REPL from executable observation

**Why:** Satisfy cleanroom rebuild requirements using only `./executable` and `docs/` as reference.

**What:**
- Original Rust source implementing a bc-like calculator REPL
- `compile.sh` producing `./executable`
- Behavior matching observed gold binary: CLI flags, expression evaluation, REPL, history, errors

**Out of scope:**
- Exact binary reproduction or byte-identical output
- Internet lookup of upstream eva source
- Decompilation or disassembly of gold binary
- Pull request to main
