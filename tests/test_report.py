import json
from pathlib import Path

from benchy.report import write_report


def test_report_table(tmp_path):
    inst = "sharkdp__hexyl.2e26437"
    for arm, rate, status in (
        ("none", 0.5, "ok"),
        ("superpowers", 1.0, "ok"),
        ("docket-superpowers", 0.0, "error:build"),
    ):
        d = tmp_path / inst / arm
        d.mkdir(parents=True)
        (d / "score.json").write_text(
            json.dumps(
                {
                    "passed": int(rate * 2),
                    "failed": 2 - int(rate * 2),
                    "total": 2,
                    "pass_rate": rate,
                    "status": status,
                    "probes": [],
                }
            )
        )
    data = write_report(tmp_path)
    assert data["tasks"][inst]["superpowers"]["resolved"] is True
    assert data["tasks"][inst]["none"]["almost"] is False
    md = (tmp_path / "report.md").read_text()
    assert "superpowers" in md
    assert inst in md
