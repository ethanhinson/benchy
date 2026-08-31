from __future__ import annotations

import subprocess
from collections.abc import Callable
from pathlib import Path

from benchy.models import TaskSpec

Runner = Callable[[str, Path], int]
Cloner = Callable[[TaskSpec, Path], None]


class GoldError(RuntimeError):
    code = "error:gold"


def _default_cloner(task: TaskSpec, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["git", "clone", task.repo, str(dest)],
        check=True,
        capture_output=True,
        text=True,
    )
    subprocess.run(
        ["git", "fetch", "--depth", "1", "origin", task.pin],
        cwd=dest,
        check=False,
        capture_output=True,
        text=True,
    )
    subprocess.run(
        ["git", "checkout", task.pin],
        cwd=dest,
        check=True,
        capture_output=True,
        text=True,
    )


def _default_runner(cmd: str, cwd: Path) -> int:
    result = subprocess.run(cmd, shell=True, cwd=cwd, check=False)
    return int(result.returncode)


def ensure_gold(
    task: TaskSpec,
    cache_dir: Path,
    *,
    runner: Runner | None = None,
    cloner: Cloner | None = None,
) -> Path:
    clone = cache_dir / task.instance_id
    binary = clone / task.gold_binary
    if binary.is_file():
        return binary
    try:
        if not clone.exists():
            (cloner or _default_cloner)(task, clone)
        code = (runner or _default_runner)(task.gold_build, clone)
        if code != 0:
            raise GoldError(f"gold build failed with {code}")
    except GoldError:
        raise
    except Exception as exc:
        raise GoldError(str(exc)) from exc
    if not binary.is_file():
        raise GoldError(f"gold binary missing: {binary}")
    return binary
