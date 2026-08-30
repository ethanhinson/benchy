from __future__ import annotations

import sys
import tarfile
from pathlib import Path

EXCLUDE_NAMES = frozenset(
    {
        "executable",
        "candidate",
        "docs",
        "RULES.md",
        "PROMPT.md",
        "skills",
        "target",
        ".git",
        "node_modules",
        "change.md",
        "spec.md",
        "plan.md",
        "review.md",
    }
)

SOURCE_NAMES = frozenset(
    {
        "compile.sh",
        "cargo.toml",
        "cargo.lock",
        "go.mod",
        "go.sum",
        "makefile",
        "cmakelists.txt",
    }
)
SOURCE_SUFFIXES = frozenset(
    {
        ".rs",
        ".c",
        ".h",
        ".cc",
        ".cpp",
        ".hpp",
        ".go",
        ".py",
        ".java",
        ".toml",
        ".sh",
    }
)


def _should_package(rel: Path) -> bool:
    if any(part in EXCLUDE_NAMES for part in rel.parts):
        return False
    if rel.name in EXCLUDE_NAMES:
        return False
    return rel.name.lower() in SOURCE_NAMES or rel.suffix.lower() in SOURCE_SUFFIXES


def package_run(run_dir: Path, dest_root: Path) -> Path:
    dest_root.mkdir(parents=True, exist_ok=True)
    found = False
    expected_arms = {"none", "superpowers", "docket-superpowers"}
    seen_arms: set[str] = set()
    for workspace in sorted(run_dir.glob("*/*/workspace")):
        found = True
        instance_id = workspace.parent.parent.name
        arm = workspace.parent.name
        seen_arms.add(arm)
        dest = dest_root / arm / instance_id
        dest.mkdir(parents=True, exist_ok=True)
        tgz = dest / "submission.tar.gz"
        with tarfile.open(tgz, "w:gz") as tf:
            for path in sorted(workspace.rglob("*")):
                if path.is_symlink() or not path.is_file():
                    continue
                rel = path.relative_to(workspace)
                if not _should_package(rel):
                    continue
                tf.add(path, arcname=str(rel))
    if not found:
        print("warning: no trial workspaces to package", file=sys.stderr)
    missing = expected_arms - seen_arms
    if missing:
        print(f"warning: missing arms: {sorted(missing)}", file=sys.stderr)
    return dest_root
