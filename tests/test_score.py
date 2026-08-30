from pathlib import Path

from benchy.models import Probe
from benchy.score import compare_outputs, score_trial


def _script(path: Path, body: str):
    path.write_text("#!/bin/sh\n" + body)
    path.chmod(0o755)


def test_score_matches_and_mismatches(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, 'if [ "$1" = --help ]; then echo help; exit 0; fi; echo no; exit 2\n')
    ws = tmp_path / "inst" / "none" / "workspace"
    ws.mkdir(parents=True)
    cand = ws / "candidate"
    _script(cand, 'if [ "$1" = --help ]; then echo help; exit 0; fi; echo yes; exit 3\n')
    probes = [
        Probe(name="help", argv=["--help"]),
        Probe(name="other", argv=["--nope"]),
    ]
    result = score_trial(workspace=ws, gold_binary=gold, probes=probes)
    assert result["passed"] == 1
    assert result["failed"] == 1
    assert result["total"] == 2
    assert result["pass_rate"] == 0.5
    assert (ws.parent / "score.json").is_file()


def test_missing_compile_is_error_build(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, "echo x\n")
    ws = tmp_path / "inst" / "none" / "workspace"
    ws.mkdir(parents=True)
    result = score_trial(
        workspace=ws, gold_binary=gold, probes=[Probe(name="h", argv=["--help"])]
    )
    assert result["status"] == "error:build"
    assert result["passed"] == 0


def test_help_normalize_is_stderr_only():
    gold = (0, "A  B\n", "usage:  x\n")
    cand = (0, "A  B\n", "usage: x")
    assert compare_outputs(gold, cand, Probe(name="h", argv=["--help"], normalize="help"))
    gold2 = (0, "A  B\n", "usage: x")
    cand2 = (0, "A B\n", "usage: x")
    assert not compare_outputs(
        gold2, cand2, Probe(name="h", argv=["--help"], normalize="help")
    )
