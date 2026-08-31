from __future__ import annotations

import json
import shutil
import stat
import warnings
from datetime import datetime, timezone
from pathlib import Path

from benchy.models import Arm, TaskSpec
from benchy.packs import copy_pack
from benchy.text import PROMPT_MD, RULES_MD

_BLOCKED_NAMES = frozenset({"src", "tests", ".git", "target", "node_modules"})


def _is_blocked(rel: Path) -> bool:
    parts = {p.lower() for p in rel.parts}
    return bool(parts & _BLOCKED_NAMES) or rel.name in _BLOCKED_NAMES


def _copy_filtered(src: Path, dest: Path) -> None:
    if src.is_symlink() or _is_blocked(Path(src.name)):
        return
    if src.is_dir():
        dest.mkdir(parents=True, exist_ok=True)
        for child in src.iterdir():
            _copy_filtered(child, dest / child.name)
        return
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)


def _safe_copy_docs(gold_docs_root: Path, dest_docs: Path, doc_paths: list[str]) -> None:
    dest_docs.mkdir(parents=True, exist_ok=True)
    root = gold_docs_root.resolve()
    for raw in doc_paths:
        src = (gold_docs_root / raw)
        if src.is_symlink():
            warnings.warn(f"skipping symlink doc path: {raw}", stacklevel=2)
            continue
        resolved = src.resolve()
        try:
            resolved.relative_to(root)
        except ValueError:
            warnings.warn(f"skipping doc path outside gold root: {raw}", stacklevel=2)
            continue
        if not resolved.exists():
            warnings.warn(f"missing doc path: {raw}", stacklevel=2)
            continue
        rel = resolved.relative_to(root)
        if _is_blocked(rel):
            warnings.warn(f"skipping blocked doc path: {raw}", stacklevel=2)
            continue
        _copy_filtered(resolved, dest_docs / rel)


def prepare_trial(
    *,
    task: TaskSpec,
    arm: Arm,
    run_dir: Path,
    packs_dir: Path,
    gold_binary: Path,
    gold_docs_root: Path,
) -> Path:
    trial_dir = run_dir / task.instance_id / arm.value
    workspace = trial_dir / "workspace"
    if workspace.exists():
        shutil.rmtree(workspace)
    workspace.mkdir(parents=True)

    dest_bin = workspace / "executable"
    shutil.copy2(gold_binary, dest_bin)
    dest_bin.chmod(stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    _safe_copy_docs(gold_docs_root, workspace / "docs", task.doc_paths)
    (workspace / "RULES.md").write_text(RULES_MD)
    (workspace / "PROMPT.md").write_text(PROMPT_MD)
    copy_pack(arm, packs_dir, workspace / "skills")

    created = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    meta = {
        "instance_id": task.instance_id,
        "slug": task.slug,
        "arm": arm.value,
        "status": "prepared",
        "created": created,
        "model": "",
    }
    (trial_dir / "trial.json").write_text(json.dumps(meta, indent=2) + "\n")
    return workspace
