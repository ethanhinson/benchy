---
name: docket-adapter
description: Docket lifecycle as local markdown only. Use when this workspace has skills/docket-adapter.
---

# Docket (local markdown)

Follow this lifecycle in this workspace only:

1. Write `change.md` — id, title, why, what, out of scope.
2. Write `spec.md` — architecture and prompt contract for the rebuild.
3. Write `plan.md` — ordered implementation steps.
4. Implement source and `compile.sh` that produces `./candidate`.
5. Write `review.md` — what was checked against `./executable`.

Superpowers skills in `./skills/` still apply.

Forbidden: `docket.sh`, remotes, GitHub PRs, metadata-branch pushes, `DOCKET_SCRIPTS_DIR`.
