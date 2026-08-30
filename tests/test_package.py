import tarfile
from pathlib import Path

from benchy.package import package_run


def test_per_arm_official_layout_excludes_gold_and_candidate(tmp_path):
    inst = "sharkdp__hexyl.2e26437"
    for arm in ("none", "superpowers"):
        ws = tmp_path / "run" / inst / arm / "workspace"
        ws.mkdir(parents=True)
        (ws / "compile.sh").write_text("#!/bin/sh\n")
        (ws / "main.rs").write_text("fn main() {}")
        (ws / "executable").write_bytes(b"gold")
        (ws / "candidate").write_bytes(b"cand")
        (ws / "RULES.md").write_text("r")
        (ws / "PROMPT.md").write_text("p")
        (ws / "docs").mkdir()
        (ws / "skills").mkdir()
    dest = tmp_path / "official"
    package_run(tmp_path / "run", dest)
    assert not (dest / inst / "submission.tar.gz").exists()
    for arm in ("none", "superpowers"):
        tgz = dest / arm / inst / "submission.tar.gz"
        assert tgz.is_file()
        with tarfile.open(tgz, "r:gz") as tf:
            names = set(tf.getnames())
        assert "compile.sh" in names
        assert "main.rs" in names
        assert "executable" not in names
        assert "candidate" not in names
        assert "RULES.md" not in names
        assert "PROMPT.md" not in names
