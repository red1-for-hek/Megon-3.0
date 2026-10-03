"""Parallel fan-out — check a whole requirements.txt at once (DESIGN §9 Extras).

Each `check()` creates its own disposable Daytona sandbox, so N concurrent checks means N
concurrent sandboxes. That is exactly what the free tier limits:

  * Daytona Tier 1 shares a **10 vCPU / 20 GiB pool across all *running* sandboxes** — about
    **10 small (1-vCPU) sandboxes at once** (DESIGN §8). Beyond that, creates queue.
  * There's also a 300 sandbox-creates/min rate limit — far above anything a fan-out hits.

So the fan-out is deliberately *bounded* — a small thread pool, not "spawn 50 at once". The
default stays a notch under the pool for headroom; override it only if event credits have
loosened your tier (the credits everyone gets on the day may — DESIGN §8).

Why threads work: checks are independent and almost entirely I/O-bound (waiting on Daytona
and the LLMs), so a bounded pool overlaps them well — 20 packages come back in roughly one
check's wall-clock, not 20×.
"""

from __future__ import annotations

import os
import re
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from typing import Callable

from .config import Config, load_config
from .gate import check
from .hook import resolve_target
from .types import Verdict

# Free-tier ceiling: the shared pool fits ~10 small sandboxes at once (DESIGN §8).
FREE_TIER_SANDBOX_CAP = 10
# Default a notch under the ceiling so a create spike doesn't overrun the pool.
DEFAULT_MAX_CONCURRENCY = 8


@dataclass
class FanoutItem:
    """One package in the fan-out: the spec as written, what got detonated, and its verdict."""

    spec: str                       # the requirement as written, e.g. "requests==2.31.0"
    target: str                     # what actually gets detonated (name, or local villain dir)
    is_local: bool
    verdict: Verdict | None = None
    error: str | None = None

    @property
    def label(self) -> str:
        if self.error:
            return "ERROR"
        return self.verdict.verdict if self.verdict else "…"


def parse_requirements(text: str) -> list[str]:
    """Pull the package specs out of requirements.txt content, in file order.

    Skips blank lines, comments, and option lines (`-r other.txt`, `-e .`, `--index-url …`),
    and strips inline comments and environment markers. A returned spec (e.g. `flask` or
    `requests==2.31.0`) is what gets handed to the gate.
    """
    specs: list[str] = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("-"):                       # -r / -e / --index-url … → not a package
            continue
        line = re.split(r"\s+#", line, maxsplit=1)[0].strip()   # drop an inline comment
        line = line.split(";", 1)[0].strip()                    # drop an environment marker
        if line:
            specs.append(line)
    return specs


def resolve_concurrency(max_concurrency: int | None) -> int:
    """Settle the sandbox concurrency: explicit arg > AIRLOCK_FANOUT_CONCURRENCY > default.

    Only the floor is clamped (>=1). An explicit value above the free-tier cap is honoured —
    the caller may have a higher tier — the CLI just warns that the free pool would queue.
    """
    if max_concurrency is None:
        env = os.getenv("AIRLOCK_FANOUT_CONCURRENCY", "").strip()
        max_concurrency = int(env) if env.isdigit() else DEFAULT_MAX_CONCURRENCY
    return max(1, max_concurrency)


def check_all(
    specs: list[str],
    *,
    config: Config | None = None,
    max_concurrency: int | None = None,
    reader: str = "auto",
    on_start: Callable[[FanoutItem], None] | None = None,
    on_done: Callable[[FanoutItem], None] | None = None,
) -> list[FanoutItem]:
    """Check every spec through the gate concurrently; return items in input order.

    At most `max_concurrency` sandboxes run at once (see module docstring for why). `on_start`
    / `on_done` fire per package (from worker threads) so a caller can show the grid filling in.
    """
    config = config or load_config()
    workers = resolve_concurrency(max_concurrency)

    items = []
    for spec in specs:
        target, is_local = resolve_target(spec)
        items.append(FanoutItem(spec=spec, target=target, is_local=is_local))

    def run(item: FanoutItem) -> None:
        if on_start:
            on_start(item)
        try:
            item.verdict = check(item.target, config=config, reader=reader)
        except Exception as e:        # one bad package must not sink the rest of the grid
            item.error = str(e)
        if on_done:
            on_done(item)

    if items:
        with ThreadPoolExecutor(max_workers=min(workers, len(items))) as pool:
            list(pool.map(run, items))   # map preserves input order; runs up to `workers` at once
    return items
