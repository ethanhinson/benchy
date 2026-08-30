# benchy

A harness for A/B testing Cursor skills on [ProgramBench](https://programbench.com/) tasks.

Same model, same task, three arms: no skills, Superpowers, and Docket+Superpowers. Agents get a gold binary and docs only. Local scoring on this Mac is a proxy; official `programbench eval` is a later Linux step, one tree per arm.

Design: [docs/superpowers/specs/2026-08-30-skill-ablation-harness-design.md](docs/superpowers/specs/2026-08-30-skill-ablation-harness-design.md)

Plan: [docs/superpowers/plans/2026-08-30-skill-ablation-harness.md](docs/superpowers/plans/2026-08-30-skill-ablation-harness.md)

## First slice

| Task | ProgramBench id |
|---|---|
| hexyl | `sharkdp__hexyl.2e26437` |
| tuc | `riquito__tuc.16fb471` |
| eva | `oppiliappan__eva.41ae245` |

## Usage

```
uv sync --extra dev
uv run pytest
uv run benchy run --slice first
```

`CURSOR_API_KEY` is required for `benchy dispatch` (install the SDK with `uv sync --extra dispatch`). `benchy run` skips dispatch when the key is absent, then still scores, packages, and reports. Local agents cannot be fully filesystem-sandboxed by the Cursor SDK; dispatch copies each trial into a temporary jail and removes the API key from the process environment for the duration of the run.

Official trees land in `runs/<run_id>-official/<arm>/<instance_id>/submission.tar.gz`.
