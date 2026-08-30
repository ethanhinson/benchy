RULES_MD = """# Cleanroom rules

You are given `./executable` and `docs/` only.

- Do not search the internet, package registries, or local caches for this project's source.
- Do not wrap, copy, chmod-as-solution, or exec `./executable` from your candidate.
- Do not decompile, disassemble, hexdump, strings, objdump, strace, or ltrace `./executable`.
- Observe behavior by running `./executable` with flags and inputs.
"""

PROMPT_MD = """# Task

Rebuild this program from `./executable` and `docs/` only.

1. Write original source and a `compile.sh` that produces `./candidate`.
2. Do not wrap, copy, or exec `./executable` in the candidate.
3. Do not decompile or disassemble `./executable`.
4. If `./skills/` exists, follow only those skills. If it does not exist, do not apply any other process skill.
"""
