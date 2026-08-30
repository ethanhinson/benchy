# Skill-ablation harness for ProgramBench

**Date:** 2026-08-30
**Status:** draft for review
**Repo:** https://github.com/ethanhinson/benchy

## Goal

Measure whether Cursor process skills change ProgramBench outcomes. Same model, same task, three arms:

| Arm | Skill pack on disk | What it tests |
|---|---|---|
| `none` | no `skills/` directory | raw agent |
| `superpowers` | curated Superpowers `SKILL.md` files | process skills |
| `docket-superpowers` | Superpowers plus a Docket process adapter | PM workflow on top of process skills |

The harness prepares isolated trial workspaces, dispatches one Cursor agent per trial, scores locally on this Mac, and packages official `submission.tar.gz` trees so `programbench eval` can run later on Linux x86_64.

This is a scaffold experiment, not a leaderboard claim, until official eval runs.

## Non-goals

- Submitting to the ProgramBench leaderboard in the first slice.
- Running official linux/amd64 cleanroom images on this Mac as the default path.
- A raw Docket install (remotes, `docket.sh` preflight, GitHub PRs) inside the trial.
- Testing every installed Cursor skill. First experiment is Superpowers and Docket only.
- Solving FFmpeg, PHP, SQLite, or other large instances.

## Architecture

A **trial** is one cell: `(task, arm)`. First slice is 3 tasks × 3 arms = 9 trials. Slice C adds more tasks; the CLI does not change.

```
catalog + packs
      │
      ▼
 prepare  →  isolated trial workspace
      │         ./executable   (gold, execute-only)
      │         docs/          (usage docs, no source)
      │         RULES.md       (ProgramBench cleanroom rules)
      │         skills/        (absent | superpowers | docket-superpowers)
      │
      ▼
 dispatch →  Cursor SDK local agent, cwd = that workspace only
      │
      ▼
  score   →  probe gold vs candidate on this Mac
      │
      ▼
 package  →  official runs/<instance>/submission.tar.gz
      │
      ▼
  report  →  per-task table: none | superpowers | docket-superpowers
```

Gold source is cloned and built **outside** the trial (experimenter cache). The agent never sees that clone. The Mac-built gold is a local-score proxy. Official `programbench eval` is a later step on Linux, using the packaged tarball.

## Components

### Task catalog

`tasks/<slug>.yaml` is the only place a ProgramBench instance is described.

Required fields:

- `instance_id` — official ProgramBench id (`owner__repo.<7-char-sha>`)
- `repo` — `https://github.com/<owner>/<repo>`
- `pin` — full or short commit matching the instance id
- `language` — `rs` for the first slice
- `gold_build` — shell command run inside the pinned clone (first slice: `cargo build --release`)
- `gold_binary` — path of the built artifact relative to the clone
- `doc_paths` — files/glob copied into the trial `docs/` (README, man pages, user-facing markdown). Source trees, tests, and `src/` are never copied.

First slice:

| Slug | Instance id | Pin |
|---|---|---|
| `hexyl` | `sharkdp__hexyl.2e26437` | `2e26437` |
| `tuc` | `riquito__tuc.16fb471` | `16fb471` |
| `eva` | `oppiliappan__eva.41ae245` | `41ae245` |

Slice C is more YAML files in the same directory, plus a `slice: expanded` tag. `benchy run --slice first` is the default; `--slice expanded` adds those tasks.

### Skill packs

`packs/<arm>/` is copied into the trial as `skills/` (except `none`, which copies nothing and does not create `skills/`).

`superpowers` vendors these files from the machine's Superpowers install at prepare time (not from GitHub at trial time):

- `brainstorming/SKILL.md`
- `writing-plans/SKILL.md`
- `test-driven-development/SKILL.md`
- `subagent-driven-development/SKILL.md`
- `systematic-debugging/SKILL.md`
- `verification-before-completion/SKILL.md`

`docket-superpowers` is that set plus `packs/docket-adapter/SKILL.md`. The adapter tells the agent to follow Docket's lifecycle as **local markdown only**: allocate a change file, write a spec, write a plan, implement, review. It forbids `docket.sh`, remotes, PRs, and metadata-branch pushes. Raw Docket skills are not copied; they cannot run in a cleanroom workspace.

Pack files are vendored into the benchy repo so trials are reproducible. Prepare refreshes them from the local install when `--refresh-packs` is set.

### Gold builder

`benchy prepare` (or the prepare phase of `run`):

1. Clone `repo` at `pin` into `.cache/gold/<instance_id>/` (gitignored).
2. Run `gold_build`.
3. Copy `gold_binary` to the trial as `./executable` and set mode `0111` (execute-only).
4. Copy `doc_paths` into `docs/`. A listed path that is missing at the pin is skipped with a warning; prepare does not fail.
5. Write `RULES.md` (ProgramBench cleanroom rules: no source lookup, no wrapping the gold binary, no decompile/disassemble/strace of `./executable`).
6. Write `PROMPT.md` (the agent instruction).
7. Copy the arm's skill pack if any.

The cache is shared across arms of the same task. One gold build per task per machine.

### Trial workspace

Layout under `runs/<run_id>/<instance_id>/<arm>/workspace/`:

```
executable
docs/
RULES.md
PROMPT.md
skills/          # omitted for arm none
compile.sh       # agent-created
src/…            # agent-created
```

Sibling metadata (not visible to the agent, or present but unused — see Dispatch): `trial.json` (task, arm, model, timestamps) lives in `runs/<run_id>/<instance_id>/<arm>/` **above** `workspace/`.

### Dispatcher

Each trial is one Cursor SDK local agent (`Agent.prompt`) with `cwd` set to that trial's `workspace/`.

- The prompt is `PROMPT.md` plus: follow only `./skills/**` if that directory exists; if it does not exist, do not follow any process skill.
- The agent must write source and a `compile.sh` that produces a candidate binary at a documented path (`./candidate` or `./compile.sh`'s stdout path). Default contract: `compile.sh` builds `./candidate`.
- Wall clock: 3 hours per trial. On timeout the trial is `error:timeout` and the slice continues.
- Host global Cursor skills are a known confound. The prompt and the workspace-only pack are the isolation mechanism. `--cloud` is a later option if local inheritance is measured to leak; it is not required for slice B.

`CURSOR_API_KEY` is required for dispatch. It is never written to disk or to `trial.json`.

### Local scorer

After dispatch (or after a human-placed `./candidate`):

1. Run `compile.sh` if `./candidate` is missing.
2. Load the task's **probe suite** — a checked-in list of `{argv, stdin, fixture}` cases under `tasks/<slug>/probes.yaml`.
3. Run each probe against the Mac gold and the candidate.
4. A probe passes when exit code, stdout, and stderr match (whitespace-normalized on stderr help text only when the probe marks `normalize: help`).
5. Write `score.json`: `{passed, failed, total, pass_rate, probes: [...]}`.

The probe suite is the same for all three arms. It is a local proxy, not the hidden ProgramBench tests. Official scores come only from `programbench eval` on Linux.

If gold and candidate both fail a probe the same way, that probe still counts as a match. If gold cannot run (arch, missing dylib), scoring for that task is `error:gold` and all three arms are skipped.

### Packager

`benchy package` writes one official-shaped tree **per arm**. Never collapse arms into a single tarball.

```
<run_id>-official/<arm>/
  <instance_id>/
    submission.tar.gz
```

`programbench eval <run_id>-official/<arm>` scores that arm only. The tarball is the agent's source tree and `compile.sh` only — not `executable`, `docs/`, `RULES.md`, `PROMPT.md`, `skills/`, or `candidate`. A missing trial is a warning; the rest still package.

### Reporter

`benchy report` prints a per-task table of local `pass_rate` by arm, plus resolved (100%) / almost (≥95%) using the local probe suite. It also writes `runs/<run_id>/report.json` and `report.md`.

## Data flow

1. Operator runs `benchy run --slice first` (or `prepare` / `dispatch` / `score` / `package` / `report` separately).
2. Catalog YAML + pack dirs + gold cache produce 9 workspaces.
3. Dispatcher launches agents sequentially by default (`--parallel 1`). `--parallel N` is allowed up to 3 (one task's three arms, or three tasks on one arm — never more than one agent per workspace).
4. Each agent exits. Scorer writes `score.json` beside the workspace.
5. Packager writes the official tree. Reporter writes the comparison.

One failed trial does not abort the slice. The report shows `error` for that cell.

## Error handling

| Failure | Posture |
|---|---|
| Catalog YAML missing a required field | Abort before any trial starts |
| Gold clone or build fails | Mark the task `error:gold`; skip all three arms |
| `CURSOR_API_KEY` missing at dispatch | Abort dispatch phase; prepared workspaces stay |
| Agent timeout or SDK error | That trial `error:timeout` / `error:dispatch`; continue |
| `compile.sh` missing or non-zero | That trial `error:build`; score 0 |
| Candidate missing after compile | That trial `error:build`; score 0 |
| Probe gold cannot execute | Task `error:gold`; skip remaining score for that task |
| Package missing a trial | Warn; package the rest |

Secrets (API keys) are env-only. `.cache/` and `runs/` are gitignored.

## Testing

Harness tests do not solve ProgramBench. They check the harness.

- **Packer leak test:** after prepare, the trial workspace contains no `src/`, no `.git` of the gold clone, and no file whose path matches the gold source tree. `executable` is mode `0111`.
- **Arm isolation test:** `none` has no `skills/`; `superpowers` has the six Superpowers files and no docket adapter; `docket-superpowers` has both.
- **Scorer fixture test:** a tiny fake gold/candidate pair with known probes produces the expected pass/fail counts.
- **Packager layout test:** output matches `<instance_id>/submission.tar.gz` and the tarball contains `compile.sh` and no gold `executable`.
- **Catalog load test:** first-slice YAML parses and instance ids match the table above.

No test may network to GitHub or Cursor. Gold-build tests are opt-in (`BENCHY_NET=1`) and not the default CI.

## CLI

```
benchy prepare --slice first [--run-id <id>]
benchy dispatch --run-id <id>
benchy score --run-id <id>
benchy package --run-id <id>
benchy report --run-id <id>
benchy run --slice first          # all five in order
```

`--slice first` = hexyl, tuc, eva. `--slice expanded` = first plus the slice C catalog (added later as YAML, not a rewrite). `--task hexyl` limits to one task. `--arm none` limits to one arm.

Implementation language: Python 3.12+ with `uv`. ProgramBench's own tooling is Python; the packager and later `programbench eval` glue stay in one ecosystem.

## Prompt contract

`PROMPT.md` states:

1. Rebuild the program from `./executable` and `docs/` only.
2. Produce `compile.sh` that writes `./candidate`.
3. Do not wrap, copy, or exec `./executable` in the candidate.
4. Do not decompile or disassemble `./executable`.
5. If `skills/` exists, follow those skills. If it does not, do not apply any other process skill.

## Success criteria

Slice B is done when:

1. `benchy run --slice first` can prepare 9 workspaces, dispatch 9 agents (or record a skipped dispatch if the key is absent), score whatever candidates exist, package official tarballs, and write a three-arm report.
2. Packer leak tests pass.
3. A Linux host can run `programbench eval` on the packaged tree without renaming files.

Slice C is adding YAML + probe files and re-running. No harness redesign.
