from pathlib import Path

from benchy.models import Probe
from benchy.score import compare_outputs, score_trial


def _script(path: Path, body: str):
    path.write_text("#!/bin/sh\n" + body)
    path.chmod(0o755)


def _compile_writes(ws: Path, body: str) -> None:
    compile_sh = ws / "compile.sh"
    compile_sh.write_text("#!/bin/sh\n" + body)
    compile_sh.chmod(0o755)


def test_score_matches_and_mismatches(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, 'if [ "$1" = --help ]; then echo help; exit 0; fi; echo no; exit 2\n')
    ws = tmp_path / "inst" / "none" / "workspace"
    ws.mkdir(parents=True)
    _compile_writes(
        ws,
        "printf '%s\\n' '#!/bin/sh' "
        "'if [ \"$1\" = --help ]; then echo help; exit 0; fi; echo yes; exit 3' "
        "> executable\nchmod +x executable\n",
    )
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


def test_compile_runs_via_shebang_not_sh(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, "echo gold\n")
    ws = tmp_path / "inst" / "none" / "workspace"
    ws.mkdir(parents=True)
    compile_sh = ws / "compile.sh"
    compile_sh.write_text(
        "#!/usr/bin/env python3\n"
        "from pathlib import Path\n"
        "p = Path('executable')\n"
        "p.write_text('#!/bin/sh\\necho ok\\n')\n"
        "p.chmod(0o755)\n"
    )
    compile_sh.chmod(0o755)
    result = score_trial(
        workspace=ws, gold_binary=gold, probes=[Probe(name="h", argv=[])]
    )
    assert result["status"] == "ok"


def test_compile_failure_records_log(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, "echo x\n")
    ws = tmp_path / "inst" / "none" / "workspace"
    ws.mkdir(parents=True)
    _compile_writes(ws, "echo boom-from-compile >&2\nexit 7\n")
    result = score_trial(
        workspace=ws, gold_binary=gold, probes=[Probe(name="h", argv=["--help"])]
    )
    assert result["status"] == "error:build"
    assert "boom-from-compile" in result["compile_log"]


def test_preexisting_gold_executable_is_not_scored(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, "echo gold\n")
    ws = tmp_path / "inst" / "none" / "workspace"
    ws.mkdir(parents=True)
    exe = ws / "executable"
    _script(exe, "echo gold\n")
    exe.chmod(0o111)
    result = score_trial(
        workspace=ws, gold_binary=gold, probes=[Probe(name="h", argv=[])]
    )
    assert result["status"] == "error:build"


def test_compile_must_write_executable_not_candidate(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, "echo gold\n")
    ws = tmp_path / "inst" / "none" / "workspace"
    ws.mkdir(parents=True)
    _compile_writes(
        ws,
        "printf '%s\\n' '#!/bin/sh' 'echo gold' > candidate\nchmod +x candidate\n",
    )
    result = score_trial(
        workspace=ws, gold_binary=gold, probes=[Probe(name="h", argv=[])]
    )
    assert result["status"] == "error:build"


def test_stale_workspace_binary_is_ignored(tmp_path):
    gold = tmp_path / "gold"
    _script(gold, "echo gold\n")
    ws = tmp_path / "inst" / "none" / "workspace"
    ws.mkdir(parents=True)
    stale = ws / "candidate"
    _script(stale, "echo gold\n")
    _compile_writes(
        ws,
        "printf '%s\\n' '#!/bin/sh' 'echo rebuilt' > executable\nchmod +x executable\n",
    )
    result = score_trial(
        workspace=ws, gold_binary=gold, probes=[Probe(name="h", argv=[])]
    )
    assert result["status"] == "ok"
    assert result["passed"] == 0
    assert result["failed"] == 1


def test_help_normalize_is_stderr_only():
    gold = (0, "A  B\n", "usage:  x\n")
    cand = (0, "A  B\n", "usage: x")
    assert compare_outputs(gold, cand, Probe(name="h", argv=["--help"], normalize="help"))
    gold2 = (0, "A  B\n", "usage: x")
    cand2 = (0, "A B\n", "usage: x")
    assert not compare_outputs(
        gold2, cand2, Probe(name="h", argv=["--help"], normalize="help")
    )
