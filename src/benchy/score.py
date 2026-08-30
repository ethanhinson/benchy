from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from pathlib import Path

import yaml

from benchy.models import Probe


def load_probes(path: Path) -> list[Probe]:
    raw = yaml.safe_load(path.read_text()) or []
    probes: list[Probe] = []
    for item in raw:
        probes.append(
            Probe(
                name=str(item["name"]),
                argv=[str(a) for a in item.get("argv", [])],
                stdin=str(item.get("stdin", "")),
                fixture=item.get("fixture"),
                normalize=item.get("normalize"),
            )
        )
    return probes


def _collapse_ws(text: str) -> str:
    return " ".join(text.split())


def run_probe(binary: Path, probe: Probe, *, workdir: Path) -> tuple[int, str, str]:
    if probe.fixture:
        src = Path(probe.fixture)
        if src.exists():
            dest = workdir / src.name
            if src.is_dir():
                shutil.copytree(src, dest, dirs_exist_ok=True)
            else:
                shutil.copy2(src, dest)
    try:
        result = subprocess.run(
            [str(binary), *probe.argv],
            input=probe.stdin,
            cwd=workdir,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except OSError:
        raise
    return result.returncode, result.stdout, result.stderr


def compare_outputs(
    gold: tuple[int, str, str],
    cand: tuple[int, str, str],
    probe: Probe,
) -> bool:
    ge, go, gs = gold
    ce, co, cs = cand
    if probe.normalize == "help":
        gs = _collapse_ws(gs)
        cs = _collapse_ws(cs)
    return ge == ce and go == co and gs == cs


def score_trial(
    *,
    workspace: Path,
    gold_binary: Path,
    probes: list[Probe],
    fixture_root: Path | None = None,
) -> dict:
    trial_dir = workspace.parent
    candidate = workspace / "candidate"
    status = "ok"
    if not candidate.is_file():
        compile_sh = workspace / "compile.sh"
        if not compile_sh.is_file():
            status = "error:build"
        else:
            compile_sh.chmod(compile_sh.stat().st_mode | 0o111)
            try:
                built = subprocess.run(
                    ["sh", str(compile_sh)],
                    cwd=workspace,
                    capture_output=True,
                    text=True,
                    timeout=300,
                )
            except subprocess.TimeoutExpired:
                status = "error:build"
            else:
                if built.returncode != 0 or not candidate.is_file():
                    status = "error:build"

    records: list[dict] = []
    passed = 0
    failed = 0
    if status == "error:build":
        result = {
            "passed": 0,
            "failed": len(probes),
            "total": len(probes),
            "pass_rate": 0.0,
            "status": status,
            "probes": records,
        }
        (trial_dir / "score.json").write_text(json.dumps(result, indent=2) + "\n")
        return result

    for probe in probes:
        resolved = probe
        if probe.fixture and fixture_root is not None:
            resolved = Probe(
                name=probe.name,
                argv=probe.argv,
                stdin=probe.stdin,
                fixture=str((fixture_root / probe.fixture).resolve()),
                normalize=probe.normalize,
            )
        with tempfile.TemporaryDirectory() as gold_wd, tempfile.TemporaryDirectory() as cand_wd:
            try:
                gold_out = run_probe(gold_binary, resolved, workdir=Path(gold_wd))
            except (OSError, subprocess.TimeoutExpired):
                result = {
                    "passed": 0,
                    "failed": 0,
                    "total": len(probes),
                    "pass_rate": 0.0,
                    "status": "error:gold",
                    "probes": records,
                }
                (trial_dir / "score.json").write_text(json.dumps(result, indent=2) + "\n")
                return result
            try:
                cand_out = run_probe(candidate, resolved, workdir=Path(cand_wd))
            except (OSError, subprocess.TimeoutExpired):
                failed += 1
                records.append(
                    {
                        "name": probe.name,
                        "match": False,
                        "gold_exit": gold_out[0],
                        "cand_exit": None,
                    }
                )
                continue
        match = compare_outputs(gold_out, cand_out, probe)
        if match:
            passed += 1
        else:
            failed += 1
        records.append(
            {
                "name": probe.name,
                "match": match,
                "gold_exit": gold_out[0],
                "cand_exit": cand_out[0],
            }
        )

    total = passed + failed
    result = {
        "passed": passed,
        "failed": failed,
        "total": total,
        "pass_rate": (passed / total) if total else 0.0,
        "status": status,
        "probes": records,
    }
    (trial_dir / "score.json").write_text(json.dumps(result, indent=2) + "\n")
    return result
