"""Tests for the parallel fan-out (airlock/fanout.py).

Offline: the gate's check() is patched, so no Daytona / keys are needed. The concurrency
cap — the one free-tier constraint that shapes the fan-out (DESIGN §8) — is checked with a
real thread race. Run with pytest, or standalone:  python tests/test_fanout.py
"""

from __future__ import annotations

import sys
import threading
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from airlock import fanout  # noqa: E402
from airlock.card import render_grid  # noqa: E402
from airlock.types import Verdict  # noqa: E402


# --- parse_requirements -----------------------------------------------------------------

def test_parse_basic():
    text = "requests\nflask==2.0\n"
    assert fanout.parse_requirements(text) == ["requests", "flask==2.0"]


def test_parse_skips_comments_blanks_and_options():
    text = "\n".join([
        "# a comment",
        "",
        "requests   # inline comment",
        "-r other.txt",
        "-e .",
        "--index-url https://example",
        "flask ; python_version >= '3.8'",
        "  six  ",
    ])
    assert fanout.parse_requirements(text) == ["requests", "flask", "six"]


# --- resolve_concurrency ----------------------------------------------------------------

def test_resolve_concurrency_default_and_override(monkeypatch=None):
    import os
    os.environ.pop("AIRLOCK_FANOUT_CONCURRENCY", None)
    assert fanout.resolve_concurrency(None) == fanout.DEFAULT_MAX_CONCURRENCY
    assert fanout.resolve_concurrency(3) == 3
    assert fanout.resolve_concurrency(0) == 1          # floor clamp
    assert fanout.resolve_concurrency(50) == 50        # explicit over-cap honoured
    os.environ["AIRLOCK_FANOUT_CONCURRENCY"] = "5"
    try:
        assert fanout.resolve_concurrency(None) == 5
    finally:
        os.environ.pop("AIRLOCK_FANOUT_CONCURRENCY", None)


# --- check_all --------------------------------------------------------------------------

def _patch_check(fake):
    saved = fanout.check
    fanout.check = fake
    return lambda: setattr(fanout, "check", saved)


def test_check_all_preserves_order_and_checks_each():
    seen = []
    lock = threading.Lock()

    def fake(target, **kw):
        with lock:
            seen.append(target)
        return Verdict(package=target, verdict="SAFE")

    restore = _patch_check(fake)
    try:
        specs = [f"pkg{i}" for i in range(10)]
        items = fanout.check_all(specs, max_concurrency=4)
    finally:
        restore()

    assert [i.spec for i in items] == specs           # returned in input order
    assert sorted(seen) == sorted(specs)              # every package was checked


def test_check_all_respects_concurrency_cap():
    active = 0
    peak = 0
    lock = threading.Lock()

    def fake(target, **kw):
        nonlocal active, peak
        with lock:
            active += 1
            peak = max(peak, active)
        time.sleep(0.03)                               # hold the slot so a race can build up
        with lock:
            active -= 1
        return Verdict(package=target, verdict="SAFE")

    restore = _patch_check(fake)
    try:
        items = fanout.check_all([f"pkg{i}" for i in range(20)], max_concurrency=4)
    finally:
        restore()

    assert peak <= 4                                   # never more than the cap in flight
    assert len(items) == 20


def test_check_all_isolates_a_failing_package():
    def fake(target, **kw):
        if target == "boom":
            raise RuntimeError("detonation failed")
        return Verdict(package=target, verdict="SAFE")

    restore = _patch_check(fake)
    try:
        items = fanout.check_all(["ok", "boom", "also-ok"], max_concurrency=3)
    finally:
        restore()

    by_spec = {i.spec: i for i in items}
    assert by_spec["boom"].error and by_spec["boom"].label == "ERROR"
    assert by_spec["ok"].label == "SAFE" and by_spec["also-ok"].label == "SAFE"


def test_check_all_resolves_local_villain():
    # The demo villain resolves to its local-index dir (so a requirements.txt line
    # `python-pillow` detonates our package, not a stranger's from PyPI).
    targets = []

    def fake(target, **kw):
        targets.append(target)
        return Verdict(package=target, verdict="BLOCK", reasons=["phoned home"])

    restore = _patch_check(fake)
    try:
        items = fanout.check_all(["python-pillow"], max_concurrency=1)
    finally:
        restore()

    assert items[0].is_local is True
    assert targets[0].endswith("python-pillow") and Path(targets[0]).is_dir()


# --- render_grid ------------------------------------------------------------------------

def test_render_grid_summarises_counts():
    items = [
        fanout.FanoutItem(spec="python-pillow", target="x", is_local=True,
                          verdict=Verdict(package="x", verdict="BLOCK", reasons=["phoned home"])),
        fanout.FanoutItem(spec="requests", target="requests", is_local=False,
                          verdict=Verdict(package="requests", verdict="SAFE")),
        fanout.FanoutItem(spec="brokenpkg", target="brokenpkg", is_local=False,
                          error="detonation failed"),
    ]
    grid = render_grid(items, color=False)
    assert "3 packages" in grid
    assert "1 BLOCK" in grid and "1 SAFE" in grid and "1 ERROR" in grid
    assert "python-pillow" in grid and "phoned home" in grid


# --- CLI fan-out rendering --------------------------------------------------------------

def test_cli_fanout_reprints_full_card_for_blocked_only():
    import contextlib
    import io
    from airlock import cli

    def fake(target, **kw):
        name = target.rstrip("/").split("/")[-1]
        if "pillow" in name:
            return Verdict(package=target, verdict="BLOCK",
                           reasons=["read a planted secret (honeytoken): ~/.env"],
                           tripwires=["read_secret"])
        return Verdict(package=name, verdict="SAFE", reasons=["No suspicious behaviour observed."])

    restore = _patch_check(fake)
    buf = io.StringIO()
    try:
        with contextlib.redirect_stdout(buf):
            code = cli.main(["six", "python-pillow"])
    finally:
        restore()

    out = buf.getvalue()
    assert code == 1                                   # a block → non-zero exit
    assert "fan-out: 2 packages" in out                # grid overview present
    # the blocked package gets a full detail card, labelled with the name the user wrote
    assert "AIRLOCK — BLOCK" in out and "package: python-pillow" in out
    assert "AIRLOCK — SAFE" not in out                 # safe ones stay compact (grid row only)


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for fn in fns:
        try:
            fn()
            print(f"  ok  {fn.__name__}")
        except Exception as e:
            failed += 1
            print(f"FAIL  {fn.__name__}: {e!r}")
    print(f"\n{len(fns) - failed}/{len(fns)} passed")
    sys.exit(1 if failed else 0)
