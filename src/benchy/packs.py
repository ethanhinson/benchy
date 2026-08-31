from __future__ import annotations

import shutil
from pathlib import Path

from benchy.models import Arm

SUPERPOWERS_SKILLS = (
    "brainstorming",
    "writing-plans",
    "test-driven-development",
    "subagent-driven-development",
    "systematic-debugging",
    "verification-before-completion",
)


def copy_pack(arm: Arm, packs_dir: Path, dest_skills_dir: Path) -> None:
    if dest_skills_dir.exists():
        shutil.rmtree(dest_skills_dir)
    if arm is Arm.NONE:
        return
    dest_skills_dir.mkdir(parents=True)
    for name in SUPERPOWERS_SKILLS:
        src = packs_dir / "superpowers" / name
        shutil.copytree(src, dest_skills_dir / name)
    if arm is Arm.DOCKET_SUPERPOWERS:
        shutil.copytree(packs_dir / "docket-adapter", dest_skills_dir / "docket-adapter")


def refresh_superpowers(packs_dir: Path, source_root: Path) -> None:
    for name in SUPERPOWERS_SKILLS:
        src = source_root / name / "SKILL.md"
        dest = packs_dir / "superpowers" / name / "SKILL.md"
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest)
