from pathlib import Path

from benchy.models import Arm
from benchy.packs import SUPERPOWERS_SKILLS, copy_pack

ROOT = Path(__file__).resolve().parents[1]


def test_none_creates_no_skills_dir(tmp_path):
    dest = tmp_path / "skills"
    copy_pack(Arm.NONE, ROOT / "packs", dest)
    assert not dest.exists()


def test_superpowers_has_six_and_no_adapter(tmp_path):
    dest = tmp_path / "skills"
    copy_pack(Arm.SUPERPOWERS, ROOT / "packs", dest)
    names = {p.name for p in dest.iterdir()}
    assert names == set(SUPERPOWERS_SKILLS)
    assert not (dest / "docket-adapter").exists()


def test_docket_arm_has_adapter_and_six(tmp_path):
    dest = tmp_path / "skills"
    copy_pack(Arm.DOCKET_SUPERPOWERS, ROOT / "packs", dest)
    text = (dest / "docket-adapter" / "SKILL.md").read_text()
    assert "docket.sh" in text
    assert "Forbidden" in text
    for name in SUPERPOWERS_SKILLS:
        assert (dest / name / "SKILL.md").is_file()


def test_copy_pack_replaces_stale_dest(tmp_path):
    dest = tmp_path / "skills"
    dest.mkdir()
    (dest / "stale").mkdir()
    (dest / "stale" / "x").write_text("old")
    copy_pack(Arm.SUPERPOWERS, ROOT / "packs", dest)
    assert not (dest / "stale").exists()
