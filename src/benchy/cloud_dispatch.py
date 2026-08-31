from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

from benchy.dispatch import DispatchError

REPO_URL = "https://github.com/ethanhinson/benchy"
ENV_JSON = """{
  "install": "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
  "start": "true"
}
"""

CLOUD_WRAP = """
This repository root is the trial workspace. Do not clone this project's original source.

Follow PROMPT.md and RULES.md. Observe `./executable` by running it.

When the rebuild is done:
1. `compile.sh` must produce `./executable` (overwrite the gold binary).
2. Commit source and `compile.sh` on this branch.
3. Do not open a pull request against main.
"""


def trial_branch(run_id: str, instance_id: str, arm: str) -> str:
    return f"trial/{run_id}/{instance_id}/{arm}"


def cloud_prompt(workspace_prompt: str) -> str:
    return workspace_prompt.rstrip() + "\n" + CLOUD_WRAP


def clone_prompt(workspace_prompt: str, *, repo_url: str, branch: str) -> str:
    https = repo_url
    if https.startswith("git@github.com:"):
        https = "https://github.com/" + https.removeprefix("git@github.com:")
    https = https.removesuffix(".git")
    return (
        f"The Cursor GitHub app cannot attach this repo. Clone the public trial branch.\n"
        f"Do not print GH_TOKEN.\n\n"
        f'git clone --branch {branch} "https://x-access-token:${{GH_TOKEN}}@{https.removeprefix("https://")}" trial\n'
        f"cd trial\n\n"
        + cloud_prompt(workspace_prompt)
        + "\nCommit and push to the same branch name. Do not create a cursor/* branch. Never print GH_TOKEN.\n"
    )


def publish_workspace(workspace: Path, *, remote: str, branch: str) -> None:
    with tempfile.TemporaryDirectory() as td:
        repo = Path(td)
        subprocess.run(["git", "init", "-b", "trial"], cwd=repo, check=True, capture_output=True)
        for item in workspace.iterdir():
            dest = repo / item.name
            if item.is_dir():
                shutil.copytree(item, dest, symlinks=False)
            else:
                mode = item.stat().st_mode
                if not os.access(item, os.R_OK):
                    item.chmod(mode | 0o400)
                    try:
                        shutil.copy2(item, dest)
                    finally:
                        item.chmod(mode)
                else:
                    shutil.copy2(item, dest)
                if item.name == "executable":
                    dest.chmod(0o755)
        env_dir = repo / ".cursor"
        env_dir.mkdir(exist_ok=True)
        (env_dir / "environment.json").write_text(ENV_JSON)
        subprocess.run(["git", "add", "-A"], cwd=repo, check=True, capture_output=True)
        subprocess.run(
            [
                "git",
                "-c",
                "user.email=benchy@local",
                "-c",
                "user.name=benchy",
                "commit",
                "-m",
                f"trial workspace {branch}",
            ],
            cwd=repo,
            check=True,
            capture_output=True,
        )
        pushed = subprocess.run(
            ["git", "push", "-u", remote, f"HEAD:{branch}"],
            cwd=repo,
            capture_output=True,
            text=True,
        )
        if pushed.returncode != 0:
            raise DispatchError(pushed.stderr or pushed.stdout, "error:dispatch")


def launch_cloud_trial(
    *,
    prompt: str,
    branch: str,
    api_key: str,
    repo_url: str = REPO_URL,
    agent_create=None,
    wait: bool = True,
) -> dict:
    from cursor_sdk import Agent, CloudAgentOptions

    create = agent_create or Agent.create
    gh_token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN") or ""
    if not gh_token:
        probed = subprocess.run(
            ["gh", "auth", "token"], capture_output=True, text=True
        )
        gh_token = (probed.stdout or "").strip()
        if gh_token:
            os.environ["GH_TOKEN"] = gh_token
    env_vars = {"GH_TOKEN": gh_token} if gh_token else None
    kwargs = dict(
        repos=[],
        auto_create_pr=False,
    )
    if env_vars:
        kwargs["env_vars"] = env_vars
    # Do not use `with Agent.create()` when wait=False. close() is local
    # cleanup; the durable cloud id lives on Agent.list(runtime=cloud).
    agent = create(
        model="composer-2.5",
        api_key=api_key,
        name=branch,
        cloud=CloudAgentOptions(**kwargs),
    )
    agent_id = getattr(agent, "agent_id", None) or getattr(agent, "id", None)
    run = agent.send(clone_prompt(prompt, repo_url=repo_url, branch=branch))
    if agent_create is None:
        cloud_id = resolve_cloud_agent_id(branch)
        if cloud_id:
            agent_id = cloud_id
    if not wait:
        return {
            "status": "running",
            "detail": "launched",
            "agent_id": str(agent_id or ""),
        }
    try:
        result = run.wait()
    finally:
        close = getattr(agent, "close", None)
        if close is not None:
            close()
    status = getattr(result, "status", None)
    if status in {"error", "cancelled"}:
        return {
            "status": "error:dispatch",
            "detail": str(result),
            "agent_id": str(agent_id or ""),
        }
    return {"status": "ok", "detail": str(status), "agent_id": str(agent_id or "")}


def resolve_cloud_agent_id(name: str, *, list_fn=None) -> str:
    from cursor_sdk import Agent

    lister = list_fn or (lambda: Agent.list({"runtime": "cloud"}))
    for item in lister():
        if getattr(item, "name", None) == name:
            return str(getattr(item, "agent_id", "") or "")
    return ""


def record_cloud_trial(trial_dir: Path, payload: dict) -> None:
    path = trial_dir / "trial.json"
    data = json.loads(path.read_text()) if path.is_file() else {}
    data.update(payload)
    path.write_text(json.dumps(data, indent=2) + "\n")


def dispatch_cloud_workspace(
    workspace: Path,
    *,
    run_id: str,
    remote: str,
    api_key: str,
    repo_url: str = REPO_URL,
    agent_create=None,
    wait: bool = True,
) -> dict:
    instance_id = workspace.parent.parent.name
    arm = workspace.parent.name
    branch = trial_branch(run_id, instance_id, arm)
    if not os.environ.get("BENCHY_SKIP_PUBLISH"):
        publish_workspace(workspace, remote=remote, branch=branch)
    prompt = (workspace / "PROMPT.md").read_text()
    result = launch_cloud_trial(
        prompt=prompt,
        branch=branch,
        api_key=api_key,
        repo_url=repo_url,
        agent_create=agent_create,
        wait=wait,
    )
    result["branch"] = branch
    record_cloud_trial(workspace.parent, result)
    return result
