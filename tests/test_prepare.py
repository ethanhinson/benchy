import json
import os
from pathlib import Path

from benchy.catalog import filter_catalog, load_catalog
from benchy.models import Arm
from benchy.prepare import prepare_trial

ROOT = Path(__file__).resolve().parents[1]


def _hexyl():
    return filter_catalog(load_catalog(ROOT / "tasks"), task="hexyl")[0]


def test_prepare_leak_and_mode(tmp_path):
    gold = tmp_path / "gold"
    gold.mkdir()
    (gold / "README.md").write_text("docs")
    src = gold / "src"
    src.mkdir()
    (src / "main.rs").write_text("fn main() {}")
    git = gold / ".git"
    git.mkdir()
    (git / "HEAD").write_text("ref")
    binary = gold / "tool"
    binary.write_bytes(b"\x7fELF")
    binary.chmod(0o755)

    ws = prepare_trial(
        task=_hexyl(),
        arm=Arm.NONE,
        run_dir=tmp_path / "run",
        packs_dir=ROOT / "packs",
        gold_binary=binary,
        gold_docs_root=gold,
    )
    assert not (ws / "src").exists()
    assert not (ws / "docs" / "src").exists()
    assert not (ws / ".git").exists()
    assert not (ws / "docs" / ".git").exists()
    assert (ws / "executable").is_file()
    assert (os.stat(ws / "executable").st_mode & 0o777) == 0o111
    assert (ws / "docs" / "README.md").is_file()
    assert (ws / "RULES.md").is_file()
    assert (ws / "PROMPT.md").is_file()
    assert not (ws / "skills").exists()
    meta = json.loads((ws.parent / "trial.json").read_text())
    assert meta["arm"] == "none"
    assert "created" in meta
    blob = json.dumps(meta).lower()
    assert "api_key" not in blob
    assert "cursor" not in blob


def test_prepare_blocks_src_even_when_listed(tmp_path):
    gold = tmp_path / "gold"
    gold.mkdir()
    (gold / "src").mkdir()
    (gold / "src" / "main.rs").write_text("secret")
    (gold / "README.md").write_text("ok")
    binary = gold / "tool"
    binary.write_bytes(b"x")
    binary.chmod(0o755)
    task = _hexyl()
    object.__setattr__(task, "doc_paths", ["README.md", "src", "../escape"])
    ws = prepare_trial(
        task=task,
        arm=Arm.NONE,
        run_dir=tmp_path / "run",
        packs_dir=ROOT / "packs",
        gold_binary=binary,
        gold_docs_root=gold,
    )
    assert not (ws / "docs" / "src").exists()
    assert (ws / "docs" / "README.md").is_file()


def test_prepare_superpowers_has_skills(tmp_path):
    gold = tmp_path / "gold"
    gold.mkdir()
    (gold / "README.md").write_text("d")
    binary = gold / "tool"
    binary.write_bytes(b"x")
    binary.chmod(0o755)
    ws = prepare_trial(
        task=_hexyl(),
        arm=Arm.SUPERPOWERS,
        run_dir=tmp_path / "run",
        packs_dir=ROOT / "packs",
        gold_binary=binary,
        gold_docs_root=gold,
    )
    assert (ws / "skills" / "brainstorming" / "SKILL.md").is_file()
