from __future__ import annotations

import json
import os
from collections.abc import Callable
from pathlib import Path

AgentFn = Callable[[str, Path, int], dict]


class DispatchError(RuntimeError):
    def __init__(self, message: str, code: str):
        super().__init__(message)
        self.code = code


def _default_agent(prompt: str, cwd: Path, timeout_s: int) -> dict:
    from cursor_sdk import Agent, AgentOptions, LocalAgentOptions

    result = Agent.prompt(
        prompt,
        AgentOptions(
            api_key=os.environ["CURSOR_API_KEY"],
            model="composer-2.5",
            local=LocalAgentOptions(cwd=str(cwd)),
        ),
    )
    status = getattr(result, "status", "ok")
    if status == "error":
        return {"status": "error:dispatch", "detail": str(result)}
    return {"status": "ok", "detail": str(status)}


def _update_trial(workspace: Path, status: str) -> None:
    path = workspace.parent / "trial.json"
    if not path.exists():
        return
    data = json.loads(path.read_text())
    data["status"] = status
    path.write_text(json.dumps(data, indent=2) + "\n")


def dispatch_trial(
    workspace: Path,
    *,
    prompt_path: Path | None = None,
    agent_fn: AgentFn | None = None,
    timeout_s: int = 10800,
) -> dict:
    key = os.environ.get("CURSOR_API_KEY")
    if agent_fn is None and not key:
        raise DispatchError("CURSOR_API_KEY is not set", "error:no_key")
    prompt = (prompt_path or workspace / "PROMPT.md").read_text()
    if key and key in prompt:
        raise DispatchError("refusing to send API key in prompt", "error:dispatch")
    fn = agent_fn or _default_agent
    try:
        result = fn(prompt, workspace, timeout_s)
    except TimeoutError as exc:
        _update_trial(workspace, "error:timeout")
        raise DispatchError(str(exc), "error:timeout") from exc
    except DispatchError:
        raise
    except Exception as exc:
        _update_trial(workspace, "error:dispatch")
        raise DispatchError(str(exc), "error:dispatch") from exc
    status = result.get("status", "ok")
    _update_trial(workspace, status)
    return result
