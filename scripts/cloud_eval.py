#!/usr/bin/env python3
"""Linux-side scoring for a packaged slice.

Builds gold binaries on this machine, unpacks each arm's submission.tar.gz,
compiles if needed, and scores against the expanded probe suites.
If `programbench` and Docker are available, also attempts official eval.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path

from benchy.catalog import load_catalog
from benchy.gold import GoldError, ensure_gold
from benchy.report import write_report
from benchy.score import load_probes, score_trial


def unpack_submission(tgz: Path, dest: Path) -> Path:
    dest.mkdir(parents=True, exist_ok=True)
    with tarfile.open(tgz, "r:gz") as tf:
        tf.extractall(dest, filter="data")
    return dest


def score_arm(root: Path, official: Path, arm: str, run_dir: Path, gold_cache: Path) -> None:
    catalog = {t.instance_id: t for t in load_catalog(root / "tasks")}
    arm_root = official / arm
    if not arm_root.is_dir():
        print(f"skip missing arm {arm}")
        return
    for inst_dir in sorted(arm_root.iterdir()):
        tgz = inst_dir / "submission.tar.gz"
        if not tgz.is_file():
            continue
        spec = catalog[inst_dir.name]
        try:
            gold = ensure_gold(spec, gold_cache)
        except GoldError as exc:
            err = run_dir / spec.instance_id / "error.json"
            err.parent.mkdir(parents=True, exist_ok=True)
            err.write_text(json.dumps({"status": "error:gold", "detail": str(exc)}) + "\n")
            print(f"{spec.instance_id}/{arm}: error:gold")
            continue
        ws = run_dir / spec.instance_id / arm / "workspace"
        if ws.exists():
            shutil.rmtree(ws)
        unpack_submission(tgz, ws)
        probes_path = root / "tasks" / spec.slug / "probes.yaml"
        probes = load_probes(probes_path)
        result = score_trial(
            workspace=ws,
            gold_binary=gold,
            probes=probes,
            fixture_root=probes_path.parent,
        )
        print(
            f"{spec.instance_id}/{arm}: {result['status']} "
            f"{result['passed']}/{result['total']} ({result['pass_rate']:.0%})"
        )


def try_official_eval(official: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    if shutil.which("docker") is None:
        (dest / "skipped.txt").write_text("docker not available\n")
        return
    try:
        subprocess.run(["uvx", "programbench", "--help"], check=True, capture_output=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        (dest / "skipped.txt").write_text("programbench not available\n")
        return
    for arm_dir in sorted(p for p in official.iterdir() if p.is_dir()):
        out = dest / arm_dir.name
        out.mkdir(parents=True, exist_ok=True)
        print(f"official eval {arm_dir.name}...")
        proc = subprocess.run(
            ["uvx", "programbench", "eval", str(arm_dir)],
            cwd=out,
            capture_output=True,
            text=True,
            timeout=7200,
        )
        (out / "eval.stdout").write_text(proc.stdout)
        (out / "eval.stderr").write_text(proc.stderr)
        (out / "exit.txt").write_text(str(proc.returncode))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--official", type=Path, default=None)
    parser.add_argument("--run-id", default="slice-b-1-linux")
    args = parser.parse_args()
    root = args.root.resolve()
    official = (args.official or root / "artifacts" / "slice-b-1-official").resolve()
    run_dir = root / "runs" / args.run_id
    gold_cache = root / ".cache" / "gold"
    run_dir.mkdir(parents=True, exist_ok=True)
    for arm in ("none", "superpowers", "docket-superpowers"):
        score_arm(root, official, arm, run_dir, gold_cache)
    write_report(run_dir)
    results = root / "docs" / "results"
    results.mkdir(parents=True, exist_ok=True)
    shutil.copy2(run_dir / "report.md", results / f"{args.run_id}.md")
    shutil.copy2(run_dir / "report.json", results / f"{args.run_id}.json")
    try_official_eval(official, results / f"{args.run_id}-programbench")
    print((run_dir / "report.md").read_text())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
