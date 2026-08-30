# benchy

A harness for A/B testing Cursor skills on [ProgramBench](https://programbench.com/) tasks.

Same model, same task, three arms: no skills, Superpowers, and Docket+Superpowers. Agents get a gold binary and docs only. Local scoring on this Mac is a proxy; official `programbench eval` is a later Linux step.

Design: [docs/superpowers/specs/2026-08-30-skill-ablation-harness-design.md](docs/superpowers/specs/2026-08-30-skill-ablation-harness-design.md)

## First slice

| Task | ProgramBench id |
|---|---|
| hexyl | `sharkdp__hexyl.2e26437` |
| tuc | `riquito__tuc.16fb471` |
| eva | `oppiliappan__eva.41ae245` |

Nine trials (3 tasks × 3 arms). An expanded slice is catalog data, not a rewrite.

## Status

Spec is in review. The CLI (`prepare` / `dispatch` / `score` / `package` / `report`) is not implemented yet.
