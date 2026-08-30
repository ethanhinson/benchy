import shutil
from pathlib import Path

from benchy.cli import main

ROOT = Path(__file__).resolve().parents[1]


def test_help_exits_zero():
    assert main(["--help"]) == 0


def test_unknown_command_exits_nonzero():
    assert main(["not-a-command"]) != 0


def test_prepare_then_report_without_network(tmp_path, monkeypatch):
    root = tmp_path / "proj"
    tasks = root / "tasks"
    tasks.mkdir(parents=True)
    shutil.copy2(ROOT / "tasks" / "hexyl.yaml", tasks / "hexyl.yaml")
    probes = tasks / "hexyl"
    probes.mkdir()
    shutil.copy2(ROOT / "tasks" / "hexyl" / "probes.yaml", probes / "probes.yaml")
    shutil.copytree(ROOT / "packs", root / "packs")

    gold_cache = tmp_path / "gold"
    clone = gold_cache / "sharkdp__hexyl.2e26437"
    bindir = clone / "target" / "release"
    bindir.mkdir(parents=True)
    (clone / "README.md").write_text("hexyl docs\n")
    hexyl = bindir / "hexyl"
    hexyl.write_text("#!/bin/sh\necho hexyl\n")
    hexyl.chmod(0o755)

    monkeypatch.setenv("BENCHY_GOLD_DIR", str(gold_cache))
    monkeypatch.delenv("CURSOR_API_KEY", raising=False)

    assert (
        main(
            [
                "--root",
                str(root),
                "run",
                "--slice",
                "first",
                "--task",
                "hexyl",
                "--run-id",
                "t1",
            ]
        )
        == 0
    )

    run = root / "runs" / "t1"
    inst = "sharkdp__hexyl.2e26437"
    for arm in ("none", "superpowers", "docket-superpowers"):
        ws = run / inst / arm / "workspace"
        assert ws.is_dir()
        assert (run / inst / arm / "trial.json").is_file()
        assert (run / inst / arm / "score.json").is_file()
        tgz = root / "runs" / "t1-official" / arm / inst / "submission.tar.gz"
        assert tgz.is_file()
    assert (run / "report.md").is_file()
    assert (run / "report.json").is_file()
    skip = (run / inst / "none" / "trial.json").read_text()
    assert "error:no_key" in skip
