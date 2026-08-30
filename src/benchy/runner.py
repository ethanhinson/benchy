from __future__ import annotations

import json
import os
import sys
from pathlib import Path

from benchy.catalog import filter_catalog, load_catalog
from benchy.dispatch import DispatchError, dispatch_trial
from benchy.gold import GoldError, ensure_gold
from benchy.models import Arm
from benchy.package import package_run
from benchy.packs import refresh_superpowers
from benchy.prepare import prepare_trial
from benchy.report import write_report
from benchy.score import load_probes, score_trial

ALL_ARMS = (Arm.NONE, Arm.SUPERPOWERS, Arm.DOCKET_SUPERPOWERS)


def iter_trials(tasks, arms: tuple[Arm, ...] = ALL_ARMS):
    for task in tasks:
        for arm in arms:
            yield task, arm


def _gold_dir(root: Path) -> Path:
    override = os.environ.get("BENCHY_GOLD_DIR")
    if override:
        return Path(override)
    return root / ".cache" / "gold"


def _packs_dir(root: Path) -> Path:
    return root / "packs"


def _run_dir(root: Path, run_id: str) -> Path:
    return root / "runs" / run_id


def run_prepare(
    root: Path,
    run_id: str,
    *,
    slice: str = "first",
    task: str | None = None,
    arm: str | None = None,
    refresh_packs: bool = False,
    refresh_source: Path | None = None,
) -> Path:
    if refresh_packs:
        raw = os.environ.get("BENCHY_SUPERPOWERS_ROOT", "")
        source = refresh_source or (Path(raw) if raw else None)
        if source is not None:
            refresh_superpowers(_packs_dir(root), source)
    tasks = filter_catalog(load_catalog(root / "tasks"), slice=slice, task=task)
    arms = ALL_ARMS if arm is None else (Arm(arm),)
    run_dir = _run_dir(root, run_id)
    run_dir.mkdir(parents=True, exist_ok=True)
    gold_cache = _gold_dir(root)
    failed_gold: set[str] = set()
    for spec, trial_arm in iter_trials(tasks, arms):
        if spec.instance_id in failed_gold:
            continue
        try:
            binary = ensure_gold(spec, gold_cache)
        except GoldError:
            failed_gold.add(spec.instance_id)
            err = run_dir / spec.instance_id / "error.json"
            err.parent.mkdir(parents=True, exist_ok=True)
            err.write_text(json.dumps({"status": "error:gold"}) + "\n")
            continue
        clone = gold_cache / spec.instance_id
        prepare_trial(
            task=spec,
            arm=trial_arm,
            run_dir=run_dir,
            packs_dir=_packs_dir(root),
            gold_binary=binary,
            gold_docs_root=clone if clone.is_dir() else binary.parent,
        )
    return run_dir


def run_dispatch(root: Path, run_id: str, *, agent_fn=None, parallel: int = 1) -> int:
    if agent_fn is None and not os.environ.get("CURSOR_API_KEY"):
        print("dispatch skipped: CURSOR_API_KEY is not set", file=sys.stderr)
        run_dir = _run_dir(root, run_id)
        for trial in sorted(run_dir.glob("*/*/trial.json")):
            data = json.loads(trial.read_text())
            data["status"] = "error:no_key"
            trial.write_text(json.dumps(data, indent=2) + "\n")
        return 0
    run_dir = _run_dir(root, run_id)
    workspaces = sorted(run_dir.glob("*/*/workspace"))
    workers = max(1, min(parallel, 3))

    def one(workspace: Path) -> None:
        try:
            dispatch_trial(workspace, agent_fn=agent_fn)
        except DispatchError as exc:
            data_path = workspace.parent / "trial.json"
            if data_path.exists():
                data = json.loads(data_path.read_text())
                data["status"] = exc.code
                data_path.write_text(json.dumps(data, indent=2) + "\n")
            print(f"{workspace}: {exc.code}", file=sys.stderr)

    if workers == 1:
        for workspace in workspaces:
            one(workspace)
    else:
        from concurrent.futures import ThreadPoolExecutor

        with ThreadPoolExecutor(max_workers=workers) as pool:
            list(pool.map(one, workspaces))
    return 0


def run_score(root: Path, run_id: str) -> Path:
    run_dir = _run_dir(root, run_id)
    gold_cache = _gold_dir(root)
    catalog = {t.instance_id: t for t in load_catalog(root / "tasks")}
    gold_failed: set[str] = set()
    for workspace in sorted(run_dir.glob("*/*/workspace")):
        instance_id = workspace.parent.parent.name
        if instance_id in gold_failed:
            continue
        spec = catalog[instance_id]
        gold_binary = gold_cache / spec.instance_id / spec.gold_binary
        probes_path = root / "tasks" / spec.slug / "probes.yaml"
        probes = load_probes(probes_path) if probes_path.is_file() else []
        result = score_trial(
            workspace=workspace,
            gold_binary=gold_binary,
            probes=probes,
            fixture_root=probes_path.parent if probes_path.is_file() else None,
        )
        if result["status"] == "error:gold":
            gold_failed.add(instance_id)
            (run_dir / instance_id / "error.json").write_text(
                json.dumps({"status": "error:gold"}) + "\n"
            )
    return run_dir


def run_package(root: Path, run_id: str) -> Path:
    dest = root / "runs" / f"{run_id}-official"
    return package_run(_run_dir(root, run_id), dest)


def run_report(root: Path, run_id: str) -> dict:
    return write_report(_run_dir(root, run_id))
