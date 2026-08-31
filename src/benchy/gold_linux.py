from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

from benchy.gold import GoldError, ensure_gold
from benchy.models import TaskSpec


def linux_gold_path(cache_dir: Path, task: TaskSpec) -> Path:
    return cache_dir / "linux-amd64" / task.instance_id / Path(task.gold_binary).name


def ensure_gold_linux(task: TaskSpec, cache_dir: Path) -> Path:
    dest = linux_gold_path(cache_dir, task)
    if dest.is_file():
        return dest
    clone = cache_dir / task.instance_id
    ensure_gold(task, cache_dir)
    dest.parent.mkdir(parents=True, exist_ok=True)
    name = Path(task.gold_binary).name
    cmd = [
        "docker",
        "run",
        "--rm",
        "--platform",
        "linux/amd64",
        "-v",
        f"{clone.resolve()}:/src:ro",
        "-v",
        f"{dest.parent.resolve()}:/out",
        "rust:1-bookworm",
        "bash",
        "-lc",
        "export PATH=/usr/local/cargo/bin:$PATH; "
        f"cp -a /src /tmp/build && cd /tmp/build && {task.gold_build} "
        f"&& cp {task.gold_binary} /out/{name}",
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0 or not dest.is_file():
        raise GoldError(proc.stderr or proc.stdout or "linux gold build failed")
    dest.chmod(0o755)
    return dest


def prepare_uses_linux_gold(root: Path) -> bool:
    return shutil.which("docker") is not None
