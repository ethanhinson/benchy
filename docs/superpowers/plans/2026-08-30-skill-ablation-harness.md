# Skill-Ablation Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Python CLI that prepares 9 ProgramBench trial workspaces (3 tasks × 3 skill arms), dispatches Cursor SDK agents, scores candidates locally, packages official `submission.tar.gz` trees, and writes a three-arm report.

**Architecture:** Pure functions over paths. Catalog YAML and skill packs are inputs. `prepare` writes isolated workspaces. `dispatch` / `score` / `package` / `report` each read `runs/<run_id>/` and write sibling artifacts. Gold clones live only in `.cache/gold/` and never enter a workspace.

**Review (2026-08-30, gpt-5.6-sol-max):** First draft rejected. Amendments below are binding: per-arm official trees (no arm picking), hardened leak tests, stderr-only help normalize, complete Task 10 test, required `slug`, secret-leak test, reconciled signatures.

**Tech Stack:** Python 3.12+, uv, pytest, PyYAML, stdlib argparse. Optional `cursor-sdk` for dispatch only.

## Global Constraints

- Python 3.12+ with uv; package name `benchy`, CLI entry `benchy`.
- No test may network to GitHub or Cursor. Gold-build tests run only when `BENCHY_NET=1`.
- `CURSOR_API_KEY` is env-only; never written to `trial.json` or any file under `runs/`.
- Arms are exactly `none`, `superpowers`, `docket-superpowers`.
- First-slice instance ids: `sharkdp__hexyl.2e26437`, `riquito__tuc.16fb471`, `oppiliappan__eva.41ae245`.
- Trial workspace path: `runs/<run_id>/<instance_id>/<arm>/workspace/`.
- Metadata path: `runs/<run_id>/<instance_id>/<arm>/trial.json` (parent of `workspace/`).
- `compile.sh` must produce `./candidate`. Gold binary in the workspace is `./executable` mode `0o111`.
- Packager tarball must contain agent source + `compile.sh` and must not contain gold `executable`.
- Work on branch `feat/harness`. Do not commit `.cache/`, `runs/`, or `.env`.

## File map

- Create: `pyproject.toml`
- Create: `src/benchy/__init__.py`
- Create: `src/benchy/__main__.py`
- Create: `src/benchy/models.py` — `Arm`, `TaskSpec`, `Probe`, `Trial`, `ScoreResult`
- Create: `src/benchy/catalog.py` — load `tasks/*.yaml`
- Create: `src/benchy/text.py` — `RULES_MD`, `PROMPT_MD`
- Create: `src/benchy/packs.py` — copy / refresh skill packs
- Create: `src/benchy/gold.py` — cache clone + build (injectable runner)
- Create: `src/benchy/prepare.py` — pack one trial workspace
- Create: `src/benchy/score.py` — compile + probe compare
- Create: `src/benchy/package.py` — official tarball tree
- Create: `src/benchy/report.py` — `report.json` + `report.md`
- Create: `src/benchy/dispatch.py` — Cursor SDK local `Agent.prompt`
- Create: `src/benchy/cli.py` — argparse
- Create: `packs/docket-adapter/SKILL.md`
- Create: `packs/superpowers/<skill>/SKILL.md` (six vendored files)
- Create: `tasks/hexyl/probes.yaml`, `tasks/tuc/probes.yaml`, `tasks/eva/probes.yaml`
- Create: `tests/conftest.py`
- Create: `tests/test_catalog.py`, `tests/test_prepare.py`, `tests/test_packs.py`, `tests/test_score.py`, `tests/test_package.py`, `tests/test_report.py`, `tests/test_dispatch.py`, `tests/test_cli.py`
- Modify: `README.md` — usage once CLI exists

---

### Task 1: Project scaffold and CLI skeleton

**Files:**
- Create: `pyproject.toml`
- Create: `src/benchy/__init__.py`
- Create: `src/benchy/__main__.py`
- Create: `src/benchy/cli.py`
- Test: `tests/test_cli.py`

**Interfaces:**
- Consumes: nothing
- Produces: `main(argv: list[str] | None = None) -> int` in `benchy.cli`. Console script `benchy`. Commands exist: `prepare`, `dispatch`, `score`, `package`, `report`, `run`. Each command may print `not implemented` and return 2 until later tasks fill them in, except `--help` which returns 0.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_cli.py
from benchy.cli import main


def test_help_exits_zero():
    assert main(["--help"]) == 0


def test_unknown_command_exits_nonzero():
    assert main(["not-a-command"]) != 0
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_cli.py -v`
Expected: FAIL with `ModuleNotFoundError` or `benchy` not installed.

- [ ] **Step 3: Write minimal implementation**

`pyproject.toml`:

```toml
[project]
name = "benchy"
version = "0.1.0"
description = "Skill-ablation harness for ProgramBench"
requires-python = ">=3.12"
dependencies = ["pyyaml>=6.0"]

[project.optional-dependencies]
dev = ["pytest>=8.0"]
dispatch = ["cursor-sdk"]

[project.scripts]
benchy = "benchy.cli:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src/benchy"]

[tool.pytest.ini_options]
testpaths = ["tests"]
```

`src/benchy/__init__.py`: `__version__ = "0.1.0"`

`src/benchy/__main__.py`: `from benchy.cli import main; raise SystemExit(main())`

`src/benchy/cli.py`: argparse with the six subcommands; `--help` uses argparse's SystemExit — catch it and return the code. `main(["--help"])` must return 0 without exiting the pytest process:

```python
import argparse
import sys


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="benchy")
    sub = p.add_subparsers(dest="cmd")
    for name in ("prepare", "dispatch", "score", "package", "report", "run"):
        sub.add_parser(name)
    return p


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as e:
        return int(e.code or 0)
    if args.cmd is None:
        parser.print_help()
        return 0
    print(f"{args.cmd}: not implemented", file=sys.stderr)
    return 2
```

- [ ] **Step 4: Run test to verify it passes**

Run: `uv sync --extra dev && uv run pytest tests/test_cli.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add pyproject.toml src/benchy tests/test_cli.py
git commit -m "feat: add benchy CLI skeleton"
```

---

### Task 2: Catalog loader

**Files:**
- Create: `src/benchy/models.py`
- Create: `src/benchy/catalog.py`
- Test: `tests/test_catalog.py`

**Interfaces:**
- Consumes: `tasks/*.yaml` at repo root (existing `hexyl.yaml`, `tuc.yaml`, `eva.yaml`)
- Produces:
  - `Arm` enum: `NONE = "none"`, `SUPERPOWERS = "superpowers"`, `DOCKET_SUPERPOWERS = "docket-superpowers"`
  - `TaskSpec` dataclass: `slug, instance_id, repo, pin, language, gold_build, gold_binary, doc_paths: list[str], slice: str`
  - `load_catalog(tasks_dir: Path) -> list[TaskSpec]` — only `tasks_dir/*.yaml` (not nested `probes.yaml`). `slug` is required. A top-level yaml that is not a valid task raises `CatalogError` (never silent skip).
  - `filter_catalog(tasks: list[TaskSpec], *, slice: str = "first", task: str | None = None) -> list[TaskSpec]`
  - `REQUIRED_FIELDS = ("slug", "instance_id", "repo", "pin", "language", "gold_build", "gold_binary", "doc_paths", "slice")`
  - Missing required field raises `CatalogError` (subclass of `ValueError`) before any trial starts.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_catalog.py
from pathlib import Path
import pytest
from benchy.catalog import CatalogError, filter_catalog, load_catalog

ROOT = Path(__file__).resolve().parents[1]


def test_first_slice_instance_ids():
    tasks = load_catalog(ROOT / "tasks")
    first = filter_catalog(tasks, slice="first")
    ids = {t.instance_id for t in first}
    assert ids == {
        "sharkdp__hexyl.2e26437",
        "riquito__tuc.16fb471",
        "oppiliappan__eva.41ae245",
    }


def test_missing_field_raises(tmp_path):
    (tmp_path / "bad.yaml").write_text("slug: bad\ninstance_id: x\n")
    with pytest.raises(CatalogError):
        load_catalog(tmp_path)


def test_filter_one_task():
    tasks = load_catalog(ROOT / "tasks")
    only = filter_catalog(tasks, slice="first", task="hexyl")
    assert [t.slug for t in only] == ["hexyl"]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_catalog.py -v`
Expected: FAIL `ModuleNotFoundError: benchy.catalog`

- [ ] **Step 3: Write minimal implementation**

`load_catalog` reads every `*.yaml` directly under `tasks_dir` (not `probes.yaml` in subdirs). Ignore files that are not mappings with `slug`. Validate `REQUIRED_FIELDS`. `filter_catalog`: if `slice == "first"`, keep `task.slice == "first"`; if `slice == "expanded"`, keep all. If `task` is set, keep that slug only.

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_catalog.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/benchy/models.py src/benchy/catalog.py tests/test_catalog.py
git commit -m "feat: load and filter the task catalog"
```

---

### Task 3: Skill packs and docket adapter

**Files:**
- Create: `packs/docket-adapter/SKILL.md`
- Create: `src/benchy/packs.py`
- Create: `packs/superpowers/brainstorming/SKILL.md` (copy from local Superpowers install)
- Create: `packs/superpowers/writing-plans/SKILL.md`
- Create: `packs/superpowers/test-driven-development/SKILL.md`
- Create: `packs/superpowers/subagent-driven-development/SKILL.md`
- Create: `packs/superpowers/systematic-debugging/SKILL.md`
- Create: `packs/superpowers/verification-before-completion/SKILL.md`
- Test: `tests/test_packs.py`

**Interfaces:**
- Consumes: `Arm`, repo `packs/` directory
- Produces:
  - `SUPERPOWERS_SKILLS = ("brainstorming", "writing-plans", "test-driven-development", "subagent-driven-development", "systematic-debugging", "verification-before-completion")`
  - `copy_pack(arm: Arm, packs_dir: Path, dest_skills_dir: Path) -> None` — for `Arm.NONE`, do not create `dest_skills_dir`. For `SUPERPOWERS`, copy the six skill dirs. For `DOCKET_SUPERPOWERS`, copy those plus `packs/docket-adapter` as `dest_skills_dir/docket-adapter/`.
  - `refresh_superpowers(packs_dir: Path, source_root: Path) -> None` — copy those six `SKILL.md` files from `source_root/<name>/SKILL.md` into `packs_dir/superpowers/<name>/`.

Docket adapter body (exact): title "Docket (local markdown)". Instruct: write `change.md`, `spec.md`, `plan.md`, then implement, then `review.md`. Forbid `docket.sh`, remotes, PRs, metadata-branch pushes. Superpowers skills in `./skills/` still apply.

Vendor the six Superpowers files from:

`/Users/EthanHinson/.cursor/plugins/cache/cursor-public/superpowers/d884ae04edebef577e82ff7c4e143debd0bbec99/skills/<name>/SKILL.md`

- [ ] **Step 1: Write the failing test**

```python
# tests/test_packs.py
from pathlib import Path
from benchy.models import Arm
from benchy.packs import SUPERPOWERS_SKILLS, copy_pack

ROOT = Path(__file__).resolve().parents[1]


def test_none_creates_no_skills_dir(tmp_path):
    dest = tmp_path / "skills"
    copy_pack(Arm.NONE, ROOT / "packs", dest)
    assert not dest.exists()


def test_superpowers_has_six_and_no_adapter(tmp_path):
    dest = tmp_path / "skills"
    copy_pack(Arm.SUPERPOWERS, ROOT / "packs", dest)
    names = {p.name for p in dest.iterdir()}
    assert names == set(SUPERPOWERS_SKILLS)
    assert not (dest / "docket-adapter").exists()


def test_docket_arm_has_adapter_and_six(tmp_path):
    dest = tmp_path / "skills"
    copy_pack(Arm.DOCKET_SUPERPOWERS, ROOT / "packs", dest)
    assert (dest / "docket-adapter" / "SKILL.md").is_file()
    for name in SUPERPOWERS_SKILLS:
        assert (dest / name / "SKILL.md").is_file()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_packs.py -v`
Expected: FAIL missing `benchy.packs` or missing pack files.

- [ ] **Step 3: Write adapter, vendor skills, implement `copy_pack`**

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_packs.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packs src/benchy/packs.py tests/test_packs.py
git commit -m "feat: vendor skill packs and copy them per arm"
```

---

### Task 4: Prepare workspace (no network)

**Files:**
- Create: `src/benchy/text.py`
- Create: `src/benchy/prepare.py`
- Test: `tests/test_prepare.py`
- Test: `tests/conftest.py`

**Interfaces:**
- Consumes: `TaskSpec`, `Arm`, `copy_pack`
- Produces:
  - `RULES_MD: str` and `PROMPT_MD: str` in `text.py` matching the spec prompt contract (rebuild from executable+docs; `compile.sh` writes `./candidate`; no wrap/copy/exec of `./executable`; no decompile; follow `./skills` only if present).
  - `prepare_trial(*, task: TaskSpec, arm: Arm, run_dir: Path, packs_dir: Path, gold_binary: Path, gold_docs_root: Path) -> Path` — returns workspace path.
  - Writes `workspace/executable` copied from `gold_binary` with mode `0o111`.
  - Copies each existing path in `task.doc_paths` from `gold_docs_root` into `workspace/docs/` (file or directory). Missing paths skipped.
  - Writes `RULES.md`, `PROMPT.md`.
  - Calls `copy_pack`.
  - Writes parent `trial.json` with keys `instance_id`, `slug`, `arm`, `status` (`"prepared"`). Must not contain `api_key` or `CURSOR`.
  - Workspace must not contain a `.git` directory and must not contain a `src/` directory copied from gold.

`conftest.py` provides `fake_gold(tmp_path)`: a file `bin/tool` (mode 0o755) and `README.md` only — no `src/`.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_prepare.py
import json
import os
from pathlib import Path
from benchy.catalog import load_catalog, filter_catalog
from benchy.models import Arm
from benchy.prepare import prepare_trial

ROOT = Path(__file__).resolve().parents[1]


def _hexyl():
    return filter_catalog(load_catalog(ROOT / "tasks"), task="hexyl")[0]


def test_prepare_leak_and_mode(tmp_path):
    gold = tmp_path / "gold"
    gold.mkdir()
    (gold / "README.md").write_text("docs")
    src = gold / "src"
    src.mkdir()
    (src / "main.rs").write_text("fn main() {}")
    binary = gold / "tool"
    binary.write_bytes(b"\x7fELF")
    binary.chmod(0o755)

    ws = prepare_trial(
        task=_hexyl(),
        arm=Arm.NONE,
        run_dir=tmp_path / "run",
        packs_dir=ROOT / "packs",
        gold_binary=binary,
        gold_docs_root=gold,
    )
    assert not (ws / "src").exists()
    assert not (ws / ".git").exists()
    assert (ws / "executable").is_file()
    assert (os.stat(ws / "executable").st_mode & 0o777) == 0o111
    assert (ws / "docs" / "README.md").is_file()
    assert (ws / "RULES.md").is_file()
    assert (ws / "PROMPT.md").is_file()
    assert not (ws / "skills").exists()
    meta = json.loads((ws.parent / "trial.json").read_text())
    assert meta["arm"] == "none"
    assert "api_key" not in json.dumps(meta).lower()


def test_prepare_superpowers_has_skills(tmp_path):
    gold = tmp_path / "gold"
    gold.mkdir()
    (gold / "README.md").write_text("d")
    binary = gold / "tool"
    binary.write_bytes(b"x")
    binary.chmod(0o755)
    ws = prepare_trial(
        task=_hexyl(),
        arm=Arm.SUPERPOWERS,
        run_dir=tmp_path / "run",
        packs_dir=ROOT / "packs",
        gold_binary=binary,
        gold_docs_root=gold,
    )
    assert (ws / "skills" / "brainstorming" / "SKILL.md").is_file()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_prepare.py -v`
Expected: FAIL missing `benchy.prepare`

- [ ] **Step 3: Implement `text.py` and `prepare.py`**

Copy binary with `shutil.copy2` then `chmod(0o111)`. Copy docs with `shutil.copy2` / `copytree`. Never copy `src/` even if listed (if a `doc_paths` entry is `src`, skip it).

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_prepare.py tests/test_packs.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/benchy/text.py src/benchy/prepare.py tests/test_prepare.py tests/conftest.py
git commit -m "feat: prepare isolated trial workspaces"
```

---

### Task 5: Gold cache (injectable, no network in default tests)

**Files:**
- Create: `src/benchy/gold.py`
- Test: `tests/test_gold.py`

**Interfaces:**
- Consumes: `TaskSpec`
- Produces:
  - `class GoldError(RuntimeError):` with `code = "error:gold"`
  - `ensure_gold(task: TaskSpec, cache_dir: Path, *, runner: Callable[[list[str], Path], int] | None = None, cloner: Callable[[TaskSpec, Path], None] | None = None) -> Path` — returns path to the built binary inside the clone (`cache_dir / task.instance_id / task.gold_binary`).
  - Default `cloner` runs `git clone --depth 1` + `git fetch` + `git checkout pin` (only used when `BENCHY_NET=1` or when a cloner is injected).
  - Default `runner` is `subprocess.run(task.gold_build, shell=True, cwd=clone, check=True)`.
  - If clone or build fails, raise `GoldError`.
  - If the binary already exists, return it (no rebuild).

- [ ] **Step 1: Write the failing test**

```python
# tests/test_gold.py
from pathlib import Path
from benchy.catalog import filter_catalog, load_catalog
from benchy.gold import GoldError, ensure_gold

ROOT = Path(__file__).resolve().parents[1]


def _hexyl():
    return filter_catalog(load_catalog(ROOT / "tasks"), task="hexyl")[0]


def test_ensure_gold_uses_injected_cloner_and_runner(tmp_path):
    task = _hexyl()

    def cloner(t, dest):
        dest.mkdir(parents=True)
        (dest / "target" / "release").mkdir(parents=True)
        (dest / "target" / "release" / "hexyl").write_bytes(b"gold")

    def runner(cmd, cwd):
        return 0

    path = ensure_gold(task, tmp_path / "cache", runner=runner, cloner=cloner)
    assert path.read_bytes() == b"gold"


def test_ensure_gold_raises_when_binary_missing(tmp_path):
    task = _hexyl()

    def cloner(t, dest):
        dest.mkdir(parents=True)

    def runner(cmd, cwd):
        return 0

    try:
        ensure_gold(task, tmp_path / "cache", runner=runner, cloner=cloner)
        raise AssertionError("expected GoldError")
    except GoldError as e:
        assert e.code == "error:gold"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_gold.py -v`
Expected: FAIL missing `benchy.gold`

- [ ] **Step 3: Implement `ensure_gold`**

Clone dest is `cache_dir / task.instance_id`. After runner, require `clone / gold_binary` exists.

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_gold.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/benchy/gold.py tests/test_gold.py
git commit -m "feat: cache gold builds with injectable clone/run"
```

---

### Task 6: Local scorer

**Files:**
- Create: `src/benchy/score.py`
- Create: `tasks/hexyl/probes.yaml`
- Create: `tasks/tuc/probes.yaml`
- Create: `tasks/eva/probes.yaml`
- Test: `tests/test_score.py`

**Interfaces:**
- Consumes: workspace path, gold binary path, `probes.yaml`
- Produces:
  - `Probe` dataclass: `name: str, argv: list[str], stdin: str = "", fixture: str | None = None, normalize: str | None = None`
  - `load_probes(path: Path) -> list[Probe]` — YAML list of mappings.
  - `run_probe(binary: Path, probe: Probe, *, workdir: Path) -> tuple[int, str, str]` — subprocess, 10s timeout, capture stdout/stderr as text.
  - `compare_outputs(gold: tuple, cand: tuple, probe: Probe) -> bool` — equal exit, stdout, stderr. If `probe.normalize == "help"`, collapse whitespace on **stderr only**. Stdout is compared raw. Gold and candidate each run in their own empty workdir copies; `probe.fixture` is a path relative to the probes file, copied into both workdirs before the run.
  - `score_trial(*, workspace: Path, gold_binary: Path, probes: list[Probe]) -> dict` — if `./candidate` missing, run `compile.sh` (must exist, exit 0, then `./candidate` must exist) else status `error:build` with `passed=0`. Write `workspace.parent / "score.json"` with `passed`, `failed`, `total`, `pass_rate`, `status`, `probes` (list of `{name, match, gold_exit, cand_exit}`).
  - If gold binary cannot execute (`OSError` or exit on spawn failure), return `status="error:gold"` and do not compare further.

First-slice `probes.yaml` for each task (same shape; namespaced by file):

```yaml
- name: help
  argv: ["--help"]
  normalize: help
- name: version
  argv: ["--version"]
  normalize: help
```

- [ ] **Step 1: Write the failing test**

```python
# tests/test_score.py
from pathlib import Path
from benchy.score import Probe, score_trial


def _script(path: Path, body: str):
    path.write_text("#!/bin/sh\n" + body)
    path.chmod(0o755)


def test_score_matches_and_mismatches(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, 'if [ "$1" = --help ]; then echo help; exit 0; fi; echo no; exit 2\n')
    ws = tmp_path / "ws"
    ws.mkdir()
    cand = ws / "candidate"
    _script(cand, 'if [ "$1" = --help ]; then echo help; exit 0; fi; echo yes; exit 3\n')
    probes = [
        Probe(name="help", argv=["--help"]),
        Probe(name="other", argv=["--nope"]),
    ]
    result = score_trial(workspace=ws, gold_binary=gold, probes=probes)
    assert result["passed"] == 1
    assert result["failed"] == 1
    assert result["total"] == 2
    assert result["pass_rate"] == 0.5
    assert (ws.parent / "score.json").is_file()


def test_missing_compile_is_error_build(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, "echo x\n")
    ws = tmp_path / "ws"
    ws.mkdir()
    result = score_trial(workspace=ws, gold_binary=gold, probes=[Probe(name="h", argv=["--help"])])
    assert result["status"] == "error:build"
    assert result["passed"] == 0
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_score.py -v`
Expected: FAIL missing `benchy.score`

- [ ] **Step 3: Implement scorer and write the three probe files**

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_score.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/benchy/score.py tasks/hexyl/probes.yaml tasks/tuc/probes.yaml tasks/eva/probes.yaml tests/test_score.py
git commit -m "feat: score candidates against gold with probe suites"
```

---

### Task 7: Packager

**Files:**
- Create: `src/benchy/package.py`
- Test: `tests/test_package.py`

**Interfaces:**
- Consumes: `run_dir: Path`
- Produces: `package_run(run_dir: Path, dest_root: Path) -> Path` — writes `dest_root / <arm> / <instance_id> / submission.tar.gz` for every trial workspace. Never pick an arm or write a collapsed `<instance_id>/submission.tar.gz`. Tarball members are agent source + `compile.sh` only. Exclude: `executable`, `docs/`, `RULES.md`, `PROMPT.md`, `skills/`, `candidate`. Missing trial → warn on stderr, continue. Return `dest_root`.

- [ ] **Step 1: Write the failing test**

```python
import tarfile
from pathlib import Path
from benchy.package import package_run


def test_per_arm_official_layout_excludes_gold_and_candidate(tmp_path):
    inst = "sharkdp__hexyl.2e26437"
    for arm in ("none", "superpowers"):
        ws = tmp_path / "run" / inst / arm / "workspace"
        ws.mkdir(parents=True)
        (ws / "compile.sh").write_text("#!/bin/sh\n")
        (ws / "main.rs").write_text("fn main() {}")
        (ws / "executable").write_bytes(b"gold")
        (ws / "candidate").write_bytes(b"cand")
        (ws / "RULES.md").write_text("r")
        (ws / "PROMPT.md").write_text("p")
        (ws / "docs").mkdir()
        (ws / "skills").mkdir()
    dest = tmp_path / "official"
    package_run(tmp_path / "run", dest)
    assert not (dest / inst / "submission.tar.gz").exists()
    for arm in ("none", "superpowers"):
        tgz = dest / arm / inst / "submission.tar.gz"
        assert tgz.is_file()
        with tarfile.open(tgz, "r:gz") as tf:
            names = set(tf.getnames())
        assert "compile.sh" in names
        assert "main.rs" in names
        assert "executable" not in names
        assert "candidate" not in names
        assert "RULES.md" not in names
        assert "PROMPT.md" not in names
```

---

### Task 8: Reporter

**Files:**
- Create: `src/benchy/report.py`
- Test: `tests/test_report.py`

**Interfaces:**
- Consumes: `run_dir` with `score.json` files
- Produces: `write_report(run_dir: Path) -> dict` — writes `run_dir/report.json` and `run_dir/report.md`. JSON shape: `{tasks: {instance_id: {arm: {pass_rate, status, resolved, almost}}}}`. `resolved` is true when `status` is ok and `pass_rate == 1.0`. `almost` is true when `pass_rate >= 0.95`. Markdown table: rows = instance ids, columns = the three arms (pass_rate or `error:...`). Missing scores show `error` / `missing`.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_report.py
import json
from pathlib import Path
from benchy.report import write_report


def test_report_table(tmp_path):
    inst = "sharkdp__hexyl.2e26437"
    for arm, rate, status in (("none", 0.5, "ok"), ("superpowers", 1.0, "ok"), ("docket-superpowers", 0.0, "error:build")):
        d = tmp_path / inst / arm
        d.mkdir(parents=True)
        (d / "score.json").write_text(json.dumps({
            "passed": int(rate * 2), "failed": 2 - int(rate * 2), "total": 2,
            "pass_rate": rate, "status": status, "probes": [],
        }))
    data = write_report(tmp_path)
    assert data["tasks"][inst]["superpowers"]["resolved"] is True
    assert data["tasks"][inst]["none"]["almost"] is False
    md = (tmp_path / "report.md").read_text()
    assert "superpowers" in md
    assert inst in md
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_report.py -v`
Expected: FAIL missing `benchy.report`

- [ ] **Step 3: Implement `write_report`**

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_report.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/benchy/report.py tests/test_report.py
git commit -m "feat: write three-arm score report"
```

---

### Task 9: Dispatcher

**Files:**
- Create: `src/benchy/dispatch.py`
- Test: `tests/test_dispatch.py`

**Interfaces:**
- Consumes: workspace path, env
- Produces:
  - `class DispatchError(RuntimeError):` with `code` of `error:dispatch` or `error:timeout` or `error:no_key`
  - `dispatch_trial(workspace: Path, *, prompt_path: Path | None = None, agent_fn=None, timeout_s: int = 10800) -> dict` — reads `PROMPT.md` (or `prompt_path`). If `CURSOR_API_KEY` is missing and `agent_fn` is None, raise `DispatchError` with `code="error:no_key"`. If `agent_fn` is provided, call `agent_fn(prompt: str, cwd: Path, timeout_s: int) -> dict` and return its dict (`status`, `detail`). Default `agent_fn` imports `cursor_sdk.Agent`, `AgentOptions`, `LocalAgentOptions` and calls `Agent.prompt(prompt, AgentOptions(api_key=..., model="composer-2.5", local=LocalAgentOptions(cwd=str(workspace))))`. Catch timeout → `error:timeout`. Catch other SDK errors → `error:dispatch`. Never write the API key. Update `trial.json` `status` to the result code.
  - CLI dispatch: missing key aborts the **phase** (exit 1) after preparing; workspaces stay.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_dispatch.py
import os
from pathlib import Path
import pytest
from benchy.dispatch import DispatchError, dispatch_trial


def test_missing_key_raises(tmp_path, monkeypatch):
    monkeypatch.delenv("CURSOR_API_KEY", raising=False)
    ws = tmp_path / "ws"
    ws.mkdir()
    (ws / "PROMPT.md").write_text("rebuild")
    with pytest.raises(DispatchError) as e:
        dispatch_trial(ws)
    assert e.value.code == "error:no_key"


def test_injected_agent(tmp_path):
    ws = tmp_path / "ws"
    ws.mkdir()
    (ws / "PROMPT.md").write_text("rebuild")
    (ws.parent / "trial.json").write_text('{"status":"prepared"}')

    def fake(prompt, cwd, timeout_s):
        assert "rebuild" in prompt
        assert cwd == ws
        return {"status": "ok", "detail": "done"}

    result = dispatch_trial(ws, agent_fn=fake)
    assert result["status"] == "ok"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_dispatch.py -v`
Expected: FAIL missing `benchy.dispatch`

- [ ] **Step 3: Implement `dispatch_trial`**

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run pytest tests/test_dispatch.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/benchy/dispatch.py tests/test_dispatch.py
git commit -m "feat: dispatch Cursor SDK agents per trial"
```

---

### Task 10: Wire CLI and `run`

**Files:**
- Modify: `src/benchy/cli.py`
- Create: `src/benchy/runner.py`
- Test: `tests/test_cli.py` (extend)
- Modify: `README.md`

**Interfaces:**
- Consumes: all prior modules
- Produces:
  - Shared flags: `--slice` (default `first`), `--task`, `--arm`, `--run-id`, `--root` (default cwd), `--refresh-packs`, `--parallel` (default 1, max 3).
  - `cmd_prepare` loads catalog, for each task×arm: `ensure_gold` (injected in tests via `runner.py` hooks or env `BENCHY_GOLD_DIR` pointing at a prebuilt cache — if `BENCHY_GOLD_DIR/<instance_id>/<gold_binary>` exists, skip clone). On `GoldError`, write `runs/<run_id>/<instance_id>/error.json` with `{"status":"error:gold"}` and skip arms.
  - `cmd_dispatch` requires key or abort phase exit 1. Sequential if `--parallel 1`. Never two agents on one workspace.
  - `cmd_score` scores every prepared trial; gold binary from cache path.
  - `cmd_package` writes `<run_id>-official/` next to `runs/<run_id>/`.
  - `cmd_report` writes report into the run dir.
  - `cmd_run` = prepare → dispatch (if key present; if absent, print skip and continue) → score → package → report. Spec success criterion: skipped dispatch if key absent, still score/package/report.

`runner.py`:

- `iter_trials(tasks, arms) -> list[tuple[TaskSpec, Arm]]`
- `run_prepare(...)`, `run_dispatch(...)`, `run_score(...)`, `run_package(...)`, `run_report(...)`

CLI test uses a temp root with a copied mini catalog and fake gold dir via `BENCHY_GOLD_DIR`.

- [ ] **Step 1: Write the failing test**

```python
# append to tests/test_cli.py
import os
from pathlib import Path
from benchy.cli import main


def test_prepare_then_report_without_network(tmp_path, monkeypatch):
    # Build a tiny root: one task yaml, packs, fake gold binary at BENCHY_GOLD_DIR
    ...
    monkeypatch.setenv("BENCHY_GOLD_DIR", str(gold_cache))
    monkeypatch.delenv("CURSOR_API_KEY", raising=False)
    assert main(["--root", str(root), "run", "--slice", "first", "--run-id", "t1"]) == 0
    assert (root / "runs" / "t1" / "report.md").is_file()
```

Implement the test fully: copy `tasks/hexyl.yaml`, `packs/`, write gold cache at `gold_cache / sharkdp__hexyl.2e26437 / target/release/hexyl`, write `tasks/hexyl/probes.yaml`, run `run` with `--task hexyl`. Expect 3 workspaces and a report.

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest tests/test_cli.py::test_prepare_then_report_without_network -v`
Expected: FAIL (run still returns 2)

- [ ] **Step 3: Implement `runner.py` and wire `cli.py`**

Update README with:

```
uv sync --extra dev
export CURSOR_API_KEY=...   # optional for dispatch
uv run benchy run --slice first
```

- [ ] **Step 4: Run all tests**

Run: `uv run pytest -v`
Expected: PASS entire suite

- [ ] **Step 5: Commit**

```bash
git add src/benchy/cli.py src/benchy/runner.py tests/test_cli.py README.md
git commit -m "feat: wire prepare/dispatch/score/package/report/run"
```

---

## Spec coverage

| Spec requirement | Task |
|---|---|
| Catalog YAML + required fields + first-slice ids | 2 |
| Three arms / pack copy / docket adapter / vendor Superpowers | 3 |
| Prepare workspace, execute-only gold, docs, RULES, PROMPT, no source leak | 4 |
| Gold cache outside trial | 5 |
| Probes, compile.sh, score.json, error:build / error:gold | 6 |
| Official submission.tar.gz without gold executable | 7 |
| Report table + resolved/almost | 8 |
| Cursor SDK dispatch, no key on disk, timeout codes | 9 |
| CLI verbs, run pipeline, skip dispatch without key | 10 |
| Slice C = more YAML | 2 (`expanded` filter already) |
| Tests do not network | all (injected gold/clone) |

## Self-review

- No TBD/TODO placeholders in task bodies.
- Types: `Arm`, `TaskSpec`, `Probe`, `GoldError.code`, `DispatchError.code` are named once and reused.
- Packager dual layout is an explicit resolution of “one official tarball per instance” vs “keep three arms”.
