from __future__ import annotations

import sys
import tarfile
from pathlib import Path

EXCLUDE = frozenset(
    {"executable", "docs", "RULES.md", "PROMPT.md", "skills", "candidate"}
)


def package_run(run_dir: Path, dest_root: Path) -> Path:
    dest_root.mkdir(parents=True, exist_ok=True)
    found = False
    for workspace in sorted(run_dir.glob("*/*/workspace")):
        found = True
        instance_id = workspace.parent.parent.name
        arm = workspace.parent.name
        dest = dest_root / arm / instance_id
        dest.mkdir(parents=True, exist_ok=True)
        tgz = dest / "submission.tar.gz"
        with tarfile.open(tgz, "w:gz") as tf:
            for path in sorted(workspace.rglob("*")):
                if not path.is_file():
                    continue
                rel = path.relative_to(workspace)
                if rel.parts[0] in EXCLUDE:
                    continue
                tf.add(path, arcname=str(rel))
    if not found:
        print("warning: no trial workspaces to package", file=sys.stderr)
    return dest_root
