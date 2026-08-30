import os
from pathlib import Path

import pytest

from benchy.dispatch import DispatchError, dispatch_trial


def test_missing_key_raises(tmp_path, monkeypatch):
    monkeypatch.delenv("CURSOR_API_KEY", raising=False)
    ws = tmp_path / "ws"
    ws.mkdir()
    (ws / "PROMPT.md").write_text("rebuild")
    with pytest.raises(DispatchError) as e:
        dispatch_trial(ws)
    assert e.value.code == "error:no_key"


def test_injected_agent(tmp_path):
    ws = tmp_path / "ws"
    ws.mkdir()
    (ws / "PROMPT.md").write_text("rebuild")
    (ws.parent / "trial.json").write_text('{"status":"prepared"}')

    def fake(prompt, cwd, timeout_s):
        assert "rebuild" in prompt
        assert cwd != ws
        (cwd / "compile.sh").write_text("built")
        return {"status": "ok", "detail": "done"}

    result = dispatch_trial(ws, agent_fn=fake)
    assert result["status"] == "ok"
    assert (ws / "compile.sh").read_text() == "built"


def test_api_key_never_written(tmp_path, monkeypatch):
    monkeypatch.setenv("CURSOR_API_KEY", "secret-sentinel-key")
    ws = tmp_path / "run" / "inst" / "none" / "workspace"
    ws.mkdir(parents=True)
    (ws / "PROMPT.md").write_text("rebuild")
    (ws.parent / "trial.json").write_text('{"status":"prepared"}')

    def fake(prompt, cwd, timeout_s):
        assert "secret-sentinel-key" not in prompt
        assert os.environ.get("CURSOR_API_KEY") is None
        return {"status": "ok", "detail": "done"}

    dispatch_trial(ws, agent_fn=fake)
    for path in tmp_path.rglob("*"):
        if path.is_file():
            assert "secret-sentinel-key" not in path.read_text()
