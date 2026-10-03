"""Command-line entry point: `python -m airlock <package>` (or `-r requirements.txt`)."""

from __future__ import annotations

import argparse
import json
import sys
import threading
from dataclasses import replace
from pathlib import Path

from .card import render_grid, render_terminal
from .fanout import (
    DEFAULT_MAX_CONCURRENCY,
    FREE_TIER_SANDBOX_CAP,
    check_all,
    parse_requirements,
    resolve_concurrency,
)
from .gate import check
from .types import Verdict


def _footer(v: Verdict) -> str | None:
    """One dim line under the card: per-stage timings + estimated cost of the check."""
    t = v.timings
    if not t.get("total"):
        return None
    parts = [f"checked in {t['total']:.0f}s"]
    # Only surface a stage that actually did work — a degraded judge returns in <1s,
    # and "read 0s · judged 0s" reads as broken rather than fast.
    ran = [k for k in ("run", "read", "judge") if t.get(k, 0) >= 1]
    labels = {"run": "ran", "read": "read", "judge": "judged"}
    if ran:
        note = ", concurrent in prod" if len(ran) > 1 else ""
        parts.append("(" + " + ".join(f"{labels[k]} {t[k]:.0f}s" for k in ran) + note + ")")
    # sandbox ≈ $0.067/hr (DESIGN §8) + ~$0.001 per LLM call that actually ran
    cost = t.get("run", 0) * 0.067 / 3600 + 0.001 * len([k for k in ("read", "judge") if t.get(k, 0) >= 1])
    parts.append("est. cost <1¢" if cost < 0.01 else f"est. cost ~${cost:.2f}")
    return "  " + " · ".join(parts)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="airlock",
        description="Detonate a package in a Daytona sandbox, read + judge it, return SAFE / BLOCK.",
    )
    parser.add_argument("package", nargs="*",
                        help="package name(s) / path(s) to check (two or more fans out)")
    parser.add_argument("-r", "--requirements", metavar="FILE",
                        help="check every package in a requirements.txt at once (parallel fan-out)")
    parser.add_argument("--concurrency", type=int, default=None, metavar="N",
                        help=f"max sandboxes at once for a fan-out (default {DEFAULT_MAX_CONCURRENCY}; "
                             f"Daytona's free-tier pool fits ~{FREE_TIER_SANDBOX_CAP}, DESIGN §8)")
    parser.add_argument("--sponsors", action="store_true",
                        help="ping every sponsor integration and report which are live (no package needed)")
    parser.add_argument("--code", action="store_true",
                        help="treat the argument as a raw Python snippet instead of a package")
    parser.add_argument("--no-static", action="store_true", help="skip the Nosana static read")
    parser.add_argument("--reader", choices=("auto", "nosana", "k"), default="auto",
                        help="static-read backend (default auto: Nosana, ai& fallback)")
    parser.add_argument("--no-judge", action="store_true", help="skip the ai& judge (tripwires only)")
    parser.add_argument("--json", action="store_true", help="print the verdict as JSON")
    args = parser.parse_args(argv)

    if args.sponsors:
        return _run_sponsors(args)

    # Assemble the package list — either from -r requirements.txt, or the positionals.
    if args.requirements:
        specs = parse_requirements(Path(args.requirements).read_text())
        if not specs:
            parser.error(f"no packages found in {args.requirements}")
    else:
        specs = args.package

    if not specs:
        parser.error("give a package to check, or -r requirements.txt")

    fanout = len(specs) > 1
    if args.code and fanout:
        parser.error("--code checks a single snippet; it can't be combined with a fan-out")

    if fanout:
        return _run_fanout(specs, args)
    return _run_single(specs[0], args)


def _run_sponsors(args) -> int:
    from .sponsors import format_report, run_checks, selfcheck_exit_code

    print("  … pinging each sponsor integration", file=sys.stderr, flush=True)
    results = run_checks()
    if args.json:
        print(json.dumps([r.__dict__ for r in results], indent=2))
    else:
        print(format_report(results, color=sys.stdout.isatty()))
    return selfcheck_exit_code(results)


def _run_single(package: str, args) -> int:
    def progress(msg: str) -> None:
        print(f"  … {msg}", file=sys.stderr, flush=True)

    v = check(
        package,
        mode="code" if args.code else "package",
        do_static=not args.no_static,
        do_judge=not args.no_judge,
        reader=args.reader,
        on_progress=progress,
    )

    if args.json:
        print(json.dumps(v.to_dict(), indent=2, default=str))
    else:
        print(render_terminal(v, color=sys.stdout.isatty()))
        footer = _footer(v)
        if footer:
            print(f"\033[2m{footer}\033[0m" if sys.stdout.isatty() else footer)
    return 1 if v.verdict == "BLOCK" else 0


def _run_fanout(specs: list[str], args) -> int:
    workers = resolve_concurrency(args.concurrency)
    print(f"  … fanning out {len(specs)} packages, up to {workers} sandboxes at once",
          file=sys.stderr, flush=True)
    if workers > FREE_TIER_SANDBOX_CAP:
        print(f"  … note: {workers} exceeds Daytona's free-tier pool (~{FREE_TIER_SANDBOX_CAP} "
              f"small sandboxes); the extra will queue unless your tier is higher (DESIGN §8)",
              file=sys.stderr, flush=True)

    lock = threading.Lock()   # on_done fires from worker threads — serialise the prints

    def on_done(item) -> None:
        mark = {"BLOCK": "✗", "SAFE": "✓", "ERROR": "!"}.get(item.label, "·")
        with lock:
            print(f"  {mark} {item.label:<5} {item.spec}", file=sys.stderr, flush=True)

    items = check_all(specs, max_concurrency=args.concurrency, reader=args.reader, on_done=on_done)

    if args.json:
        print(json.dumps(
            [{"spec": i.spec, "verdict": i.label,
              "result": i.verdict.to_dict() if i.verdict else None, "error": i.error}
             for i in items], indent=2, default=str))
    else:
        print(render_grid(items, color=sys.stdout.isatty()))
        # Safe packages are done at the grid row; for the blocked ones, print the full
        # single-package card again so the whole "why it failed" is right there.
        blocked = [i for i in items if i.label == "BLOCK" and i.verdict is not None]
        for i in blocked:
            print()
            # Show the name the user wrote (e.g. "python-pillow"), not the resolved local dir.
            print(render_terminal(replace(i.verdict, package=i.spec), color=sys.stdout.isatty()))
    return 1 if any(i.label == "BLOCK" for i in items) else 0


if __name__ == "__main__":
    sys.exit(main())
