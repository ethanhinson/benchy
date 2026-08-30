from __future__ import annotations

import argparse
from datetime import datetime, timezone
from pathlib import Path

from benchy.runner import run_dispatch, run_package, run_prepare, run_report, run_score


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="benchy")
    p.add_argument("--root", type=Path, default=None)
    sub = p.add_subparsers(dest="cmd")
    for name in ("prepare", "dispatch", "score", "package", "report", "run"):
        sp = sub.add_parser(name)
        sp.add_argument("--slice", default="first")
        sp.add_argument("--task")
        sp.add_argument("--arm")
        sp.add_argument("--run-id")
        sp.add_argument("--refresh-packs", action="store_true")
        sp.add_argument("--parallel", type=int, default=1)
    return p


def _run_id(args) -> str:
    return args.run_id or datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def _root(args) -> Path:
    return (args.root or Path.cwd()).resolve()


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as e:
        return int(e.code or 0)
    if args.cmd is None:
        parser.print_help()
        return 0
    if getattr(args, "parallel", 1) > 3:
        print("error: --parallel max is 3", file=__import__("sys").stderr)
        return 2
    root = _root(args)
    run_id = _run_id(args)
    if args.cmd == "prepare":
        run_prepare(
            root,
            run_id,
            slice=args.slice,
            task=args.task,
            arm=args.arm,
            refresh_packs=args.refresh_packs,
        )
        return 0
    if args.cmd == "dispatch":
        if not __import__("os").environ.get("CURSOR_API_KEY"):
            print("dispatch aborted: CURSOR_API_KEY is not set", file=__import__("sys").stderr)
            return 1
        return run_dispatch(root, run_id, parallel=args.parallel)
    if args.cmd == "score":
        run_score(root, run_id)
        return 0
    if args.cmd == "package":
        run_package(root, run_id)
        return 0
    if args.cmd == "report":
        run_report(root, run_id)
        return 0
    if args.cmd == "run":
        run_prepare(
            root,
            run_id,
            slice=args.slice,
            task=args.task,
            arm=args.arm,
            refresh_packs=args.refresh_packs,
        )
        run_dispatch(root, run_id, parallel=args.parallel)
        run_score(root, run_id)
        run_package(root, run_id)
        run_report(root, run_id)
        return 0
    return 2
