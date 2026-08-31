from __future__ import annotations

import json
import os
import shutil
import tempfile
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

AgentFn = Callable[[str, Path, int], dict]


class DispatchError(RuntimeError):
    def __init__(self, message: str, code: str):
        super().__init__(message)
        self.code = code


def _default_agent(prompt: str, cwd: Path, timeout_s: int, api_key: str = "") -> dict:
    from cursor_sdk import Agent, AgentOptions, LocalAgentOptions

    kwargs: dict = {
        "api_key": api_key,
        "model": "composer-2.5",
        "local": LocalAgentOptions(cwd=str(cwd)),
    }

    def call():
        return Agent.prompt(prompt, AgentOptions(**kwargs))

    with ThreadPoolExecutor(max_workers=1) as pool:
        future = pool.submit(call)
        result = future.result(timeout=timeout_s)
    status = getattr(result, "status", None)
    if status in {"error", "cancelled"}:
        return {"status": "error:dispatch", "detail": str(result)}
    return {"status": "ok", "detail": str(status)}


def _update_trial(workspace: Path, status: str) -> None:
    path = workspace.parent / "trial.json"
    if not path.exists():
        return
    data = json.loads(path.read_text())
    data["status"] = status
    path.write_text(json.dumps(data, indent=2) + "\n")


def _copy_maybe_exec_only(src: Path, dest: Path) -> None:
    mode = src.stat().st_mode
    added_read = False
    if not os.access(src, os.R_OK):
        src.chmod(mode | 0o400)
        added_read = True
    try:
        shutil.copy2(src, dest)
    finally:
        if added_read:
            src.chmod(mode)
    dest.chmod(mode)


def _jail_workspace(workspace: Path) -> Path:
    jail = Path(tempfile.mkdtemp(prefix="benchy-jail-"))
    for item in workspace.iterdir():
        dest = jail / item.name
        if item.is_dir():
            shutil.copytree(item, dest, symlinks=False)
        else:
            _copy_maybe_exec_only(item, dest)
    return jail


def _copy_jail_back(jail: Path, workspace: Path) -> None:
    for item in jail.iterdir():
        if item.name == "executable":
            continue
        dest = workspace / item.name
        if dest.exists():
            if dest.is_dir():
                shutil.rmtree(dest)
            else:
                dest.unlink()
        if item.is_dir():
            shutil.copytree(item, dest, symlinks=False)
        else:
            shutil.copy2(item, dest)


def dispatch_trial(
    workspace: Path,
    *,
    prompt_path: Path | None = None,
    agent_fn: AgentFn | None = None,
    timeout_s: int = 10800,
    api_key: str | None = None,
) -> dict:
    key = api_key if api_key is not None else os.environ.get("CURSOR_API_KEY")
    if agent_fn is None and not key:
        raise DispatchError("CURSOR_API_KEY is not set", "error:no_key")
    prompt = (prompt_path or workspace / "PROMPT.md").read_text()
    if key and key in prompt:
        raise DispatchError("refusing to send API key in prompt", "error:dispatch")
    saved_key = None
    if api_key is None:
        saved_key = os.environ.pop("CURSOR_API_KEY", None)
    fn = agent_fn or (
        lambda prompt, cwd, timeout: _default_agent(
            prompt, cwd, timeout, api_key=key or ""
        )
    )
    jail = _jail_workspace(workspace)
    try:
        try:
            result = fn(prompt, jail, timeout_s)
        except TimeoutError as exc:
            _update_trial(workspace, "error:timeout")
            raise DispatchError(str(exc), "error:timeout") from exc
        except DispatchError:
            raise
        except Exception as exc:
            _update_trial(workspace, "error:dispatch")
            raise DispatchError(str(exc), "error:dispatch") from exc
        _copy_jail_back(jail, workspace)
        status = result.get("status", "ok")
        _update_trial(workspace, status)
        return result
    finally:
        if saved_key is not None:
            os.environ["CURSOR_API_KEY"] = saved_key
        shutil.rmtree(jail, ignore_errors=True)
