from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class Arm(str, Enum):
    NONE = "none"
    SUPERPOWERS = "superpowers"
    DOCKET_SUPERPOWERS = "docket-superpowers"


@dataclass(frozen=True)
class TaskSpec:
    slug: str
    instance_id: str
    repo: str
    pin: str
    language: str
    gold_build: str
    gold_binary: str
    doc_paths: list[str]
    slice: str


@dataclass(frozen=True)
class Probe:
    name: str
    argv: list[str]
    stdin: str = ""
    fixture: str | None = None
    normalize: str | None = None


@dataclass
class Trial:
    task: TaskSpec
    arm: Arm
    workspace: str
    status: str = "prepared"
    created: str = ""
    model: str = ""
