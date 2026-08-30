from __future__ import annotations

import json
from pathlib import Path

ARMS = ("none", "superpowers", "docket-superpowers")


def write_report(run_dir: Path) -> dict:
    tasks: dict[str, dict] = {}
    for err_path in sorted(run_dir.glob("*/error.json")):
        instance_id = err_path.parent.name
        raw = json.loads(err_path.read_text())
        status = str(raw.get("status", "error:gold"))
        cell = {
            "pass_rate": 0.0,
            "status": status,
            "resolved": False,
            "almost": False,
        }
        tasks.setdefault(instance_id, {})
        for arm in ARMS:
            tasks[instance_id].setdefault(arm, cell)
    for score_path in sorted(run_dir.glob("*/*/score.json")):
        instance_id = score_path.parent.parent.name
        arm = score_path.parent.name
        raw = json.loads(score_path.read_text())
        rate = float(raw.get("pass_rate", 0.0))
        status = str(raw.get("status", "ok"))
        cell = {
            "pass_rate": rate,
            "status": status,
            "resolved": status == "ok" and rate == 1.0,
            "almost": status == "ok" and rate >= 0.95,
        }
        tasks.setdefault(instance_id, {})[arm] = cell

    for instance_id, arms in tasks.items():
        for arm in ARMS:
            arms.setdefault(
                arm,
                {
                    "pass_rate": 0.0,
                    "status": "missing",
                    "resolved": False,
                    "almost": False,
                },
            )

    data = {"tasks": tasks}
    (run_dir / "report.json").write_text(json.dumps(data, indent=2) + "\n")

    lines = [
        "| instance | none | superpowers | docket-superpowers |",
        "|---|---|---|---|",
    ]
    for instance_id in sorted(tasks):
        cells = []
        for arm in ARMS:
            cell = tasks[instance_id][arm]
            if cell["status"] not in {"ok"}:
                cells.append(cell["status"])
            else:
                cells.append(f"{cell['pass_rate']:.1%}")
        lines.append(f"| {instance_id} | {cells[0]} | {cells[1]} | {cells[2]} |")
    (run_dir / "report.md").write_text("\n".join(lines) + "\n")
    return data
