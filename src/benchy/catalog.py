from __future__ import annotations

from pathlib import Path

import yaml

from benchy.models import TaskSpec

REQUIRED_FIELDS = (
    "slug",
    "instance_id",
    "repo",
    "pin",
    "language",
    "gold_build",
    "gold_binary",
    "doc_paths",
    "slice",
)


class CatalogError(ValueError):
    pass


def load_catalog(tasks_dir: Path) -> list[TaskSpec]:
    tasks: list[TaskSpec] = []
    for path in sorted(tasks_dir.glob("*.yaml")):
        raw = yaml.safe_load(path.read_text())
        if not isinstance(raw, dict):
            raise CatalogError(f"{path.name}: expected a mapping")
        missing = [f for f in REQUIRED_FIELDS if f not in raw]
        if missing:
            raise CatalogError(f"{path.name}: missing {missing}")
        doc_paths = raw["doc_paths"]
        if not isinstance(doc_paths, list):
            raise CatalogError(f"{path.name}: doc_paths must be a list")
        tasks.append(
            TaskSpec(
                slug=str(raw["slug"]),
                instance_id=str(raw["instance_id"]),
                repo=str(raw["repo"]),
                pin=str(raw["pin"]),
                language=str(raw["language"]),
                gold_build=str(raw["gold_build"]),
                gold_binary=str(raw["gold_binary"]),
                doc_paths=[str(p) for p in doc_paths],
                slice=str(raw["slice"]),
            )
        )
    return tasks


def filter_catalog(
    tasks: list[TaskSpec],
    *,
    slice: str = "first",
    task: str | None = None,
) -> list[TaskSpec]:
    if slice == "first":
        selected = [t for t in tasks if t.slice == "first"]
    elif slice == "expanded":
        selected = list(tasks)
    else:
        raise CatalogError(f"unknown slice: {slice}")
    if task is not None:
        selected = [t for t in selected if t.slug == task]
        if not selected:
            raise CatalogError(f"unknown task: {task}")
    return selected
