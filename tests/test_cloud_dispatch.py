from pathlib import Path
import subprocess

from benchy.cloud_dispatch import cloud_prompt, publish_workspace, trial_branch


def test_trial_branch_name():
    assert (
        trial_branch("slice-c-1", "sharkdp__hexyl.2e26437", "none")
        == "trial/slice-c-1/sharkdp__hexyl.2e26437/none"
    )


def test_cloud_prompt_uses_workspace_prompt():
    text = cloud_prompt("rebuild from executable\n")
    assert "rebuild from executable" in text
    assert "this repository root is the trial workspace" in text.lower()


def test_clone_prompt_does_not_inline_token():
    from benchy.cloud_dispatch import clone_prompt

    text = clone_prompt(
        "rebuild\n",
        repo_url="https://github.com/ethanhinson/benchy",
        branch="trial/slice-c-1/hexyl/none",
    )
    assert "trial/slice-c-1/hexyl/none" in text
    assert "secret" not in text
    assert "${GH_TOKEN}" in text or "$GH_TOKEN" in text


class _FakeRun:
    def wait(self):
        return type("R", (), {"status": "ok"})()


class _FakeAgent:
    def __init__(self):
        self.agent_id = "bc-test"
        self.closed = False
        self.sent = 0

    def send(self, prompt):
        self.sent += 1
        assert "GH_TOKEN" in prompt or "${GH_TOKEN}" in prompt or "$GH_TOKEN" in prompt
        return _FakeRun()

    def close(self):
        self.closed = True

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()


def test_resolve_cloud_agent_id_matches_name():
    from benchy.cloud_dispatch import resolve_cloud_agent_id

    class Item:
        def __init__(self, name, agent_id):
            self.name = name
            self.agent_id = agent_id

    found = resolve_cloud_agent_id(
        "trial/slice-c-1/hexyl/none",
        list_fn=lambda: [
            Item("other", "bc-1"),
            Item("trial/slice-c-1/hexyl/none", "bc-real"),
        ],
    )
    assert found == "bc-real"


def test_launch_wait_false_does_not_close(monkeypatch):
    from benchy.cloud_dispatch import launch_cloud_trial

    agent = _FakeAgent()
    monkeypatch.setenv("GH_TOKEN", "not-a-real-token")
    result = launch_cloud_trial(
        prompt="rebuild\n",
        branch="trial/test/hexyl/none",
        api_key="k",
        agent_create=lambda **kwargs: agent,
        wait=False,
    )
    assert result["status"] == "running"
    assert result["agent_id"] == "bc-test"
    assert agent.sent == 1
    assert agent.closed is False


def test_launch_wait_true_closes():
    from benchy.cloud_dispatch import launch_cloud_trial

    agent = _FakeAgent()
    result = launch_cloud_trial(
        prompt="rebuild\n",
        branch="trial/test/hexyl/none",
        api_key="k",
        agent_create=lambda **kwargs: agent,
        wait=True,
    )
    assert result["status"] == "ok"
    assert agent.closed is True


def test_publish_workspace_to_local_remote(tmp_path):
    ws = tmp_path / "workspace"
    ws.mkdir()
    (ws / "PROMPT.md").write_text("rebuild\n")
    (ws / "RULES.md").write_text("rules\n")
    remote = tmp_path / "remote.git"
    subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
    branch = "trial/test/hexyl/none"
    publish_workspace(ws, remote=str(remote), branch=branch)
    listed = subprocess.run(
        ["git", "--git-dir", str(remote), "branch", "--list", branch],
        check=True,
        capture_output=True,
        text=True,
    )
    assert branch in listed.stdout
