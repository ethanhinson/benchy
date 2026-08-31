from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

import yaml

from benchy.models import Probe
from benchy.package import should_package

COMPILE_TIMEOUT_S = 900


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


def stage_submission(workspace: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    for path in workspace.rglob("*"):
        if path.is_symlink() or not path.is_file():
            continue
        rel = path.relative_to(workspace)
        if not should_package(rel):
            continue
        target = dest / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, target)


def _seed_git(staged: Path) -> None:
    if (staged / ".git").exists():
        return
    env = os.environ.copy()
    env["GIT_AUTHOR_DATE"] = "2000-01-01T00:00:00Z"
    env["GIT_COMMITTER_DATE"] = "2000-01-01T00:00:00Z"
    try:
        subprocess.run(
            ["git", "-c", "init.defaultBranch=gold", "init", "-q"],
            cwd=staged,
            check=True,
            capture_output=True,
            env=env,
        )
        subprocess.run(
            [
                "git",
                "-c",
                "user.email=gold@local",
                "-c",
                "user.name=gold",
                "-c",
                "commit.gpgsign=false",
                "add",
                "-A",
            ],
            cwd=staged,
            check=True,
            capture_output=True,
            env=env,
        )
        subprocess.run(
            [
                "git",
                "-c",
                "user.email=gold@local",
                "-c",
                "user.name=gold",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "gold",
            ],
            cwd=staged,
            check=True,
            capture_output=True,
            env=env,
        )
    except (FileNotFoundError, subprocess.CalledProcessError):
        return


def compile_submission(staged: Path) -> tuple[Path | None, str]:
    official = staged / "executable"
    official.unlink(missing_ok=True)
    compile_sh = staged / "compile.sh"
    if not compile_sh.is_file():
        return None, ""
    _seed_git(staged)
    compile_sh.chmod(compile_sh.stat().st_mode | 0o111)
    try:
        built = subprocess.run(
            [str(compile_sh.resolve())],
            cwd=staged,
            capture_output=True,
            text=True,
            timeout=COMPILE_TIMEOUT_S,
        )
    except subprocess.TimeoutExpired as exc:
        out = exc.stdout or ""
        err = exc.stderr or ""
        return None, f"{out}{err}\ntimeout after {COMPILE_TIMEOUT_S}s"
    log = f"{built.stdout or ''}{built.stderr or ''}"
    if built.returncode != 0 or not official.is_file():
        return None, log
    return official, log


def score_trial(
    *,
    workspace: Path,
    gold_binary: Path,
    probes: list[Probe],
    fixture_root: Path | None = None,
) -> dict:
    trial_dir = workspace.parent
    compile_log = ""
    status = "ok"
    built: Path | None = None
    with tempfile.TemporaryDirectory() as td:
        staged = Path(td) / "submission"
        stage_submission(workspace, staged)
        built, compile_log = compile_submission(staged)
        if built is None:
            status = "error:build"
            result = {
                "passed": 0,
                "failed": len(probes),
                "total": len(probes),
                "pass_rate": 0.0,
                "status": status,
                "probes": [],
                "compile_log": compile_log[-8000:],
            }
            (trial_dir / "score.json").write_text(json.dumps(result, indent=2) + "\n")
            return result

        keep = Path(td) / "built"
        shutil.copy2(built, keep)
        keep.chmod(keep.stat().st_mode | 0o111)
        records: list[dict] = []
        passed = 0
        failed = 0
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
                    (trial_dir / "score.json").write_text(
                        json.dumps(result, indent=2) + "\n"
                    )
                    return result
                try:
                    cand_out = run_probe(keep, resolved, workdir=Path(cand_wd))
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
