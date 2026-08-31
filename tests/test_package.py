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
        (ws / "change.md").write_text("process")
        (ws / "docs").mkdir()
        (ws / "docs" / "README.md").write_text("docs")
        (ws / "skills").mkdir()
        (ws / "skills" / "x.md").write_text("skill")
        (ws / "target" / "release").mkdir(parents=True)
        (ws / "target" / "release" / "hexyl").write_bytes(b"bin")
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
        assert "change.md" not in names
        assert not any(n.startswith("target/") for n in names)
        assert not any(n.startswith("docs/") for n in names)


def test_package_includes_vendored_crates(tmp_path):
    inst = "sharkdp__hexyl.2e26437"
    ws = tmp_path / "run" / inst / "none" / "workspace"
    (ws / "vendor" / "clap-4.6.6").mkdir(parents=True)
    (ws / "src").mkdir(parents=True)
    (ws / "compile.sh").write_text("#!/bin/sh\n")
    (ws / "src" / "main.rs").write_text("fn main() {}\n")
    (ws / "vendor" / "clap-4.6.6" / "LICENSE").write_text("MIT\n")
    (ws / ".cargo").mkdir()
    (ws / ".cargo" / "config.toml").write_text("[source.crates-io]\n")
    dest = tmp_path / "official"
    package_run(tmp_path / "run", dest)
    with tarfile.open(dest / "none" / inst / "submission.tar.gz", "r:gz") as tf:
        names = set(tf.getnames())
    assert "vendor/clap-4.6.6/LICENSE" in names
    assert ".cargo/config.toml" in names


def test_tracked_official_trees_match_eval_layout():
    root = Path(__file__).resolve().parents[1] / "artifacts" / "slice-b-1-official"
    instances = (
        "sharkdp__hexyl.2e26437",
        "riquito__tuc.16fb471",
        "oppiliappan__eva.41ae245",
    )
    for arm in ("none", "superpowers", "docket-superpowers"):
        for inst in instances:
            tgz = root / arm / inst / "submission.tar.gz"
            assert tgz.is_file(), tgz
            with tarfile.open(tgz, "r:gz") as tf:
                names = set(tf.getnames())
                compile_sh = tf.extractfile("compile.sh")
                assert compile_sh is not None
                body = compile_sh.read().decode()
            assert "compile.sh" in names, tgz
            assert "executable" not in names, tgz
            assert "./executable" in body, tgz
            assert "--offline" in body, tgz
            assert ".cargo/config.toml" in names, tgz
            assert any(n.startswith("vendor/") for n in names), tgz
