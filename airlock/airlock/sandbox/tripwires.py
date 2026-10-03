"""Step 1 — the deterministic tripwires (Tier 1 of the verdict, no model).

Some actions are never OK during a simple install. If any fires, it's an instant BLOCK
in plain code — this is the floor the demo rides on, and the LLM judge can only make the
gate stricter, never overrule a tripwire.
"""

from __future__ import annotations

import re

from ..types import Event

TRIPWIRE_LABELS = {
    "read_secret": "Secretly read private files we'd planted as bait",
    "connect": "Tried to send data out to an unknown server",
    "spawn_shell": "Tried to run shell commands on the machine",
}

# Collapse the sandbox home prefix to ~ for readable output (/home/daytona/.aws → ~/.aws).
_HOME_RE = re.compile(r"^/(?:home/[^/]+|root)/")


def _short(detail: str) -> str:
    return _HOME_RE.sub("~/", detail)


def evaluate_tripwires(events: list[Event]) -> tuple[bool, list[str], list[str]]:
    """Return (blocked, fired_kinds, human_reasons)."""
    fired: list[str] = []
    for e in events:
        if e.kind in TRIPWIRE_LABELS and e.kind not in fired:
            fired.append(e.kind)

    reasons: list[str] = []
    for kind in fired:
        details = [_short(e.detail) for e in events if e.kind == kind]
        shown = ", ".join(details[:3])
        reasons.append(f"{TRIPWIRE_LABELS[kind]}: {shown}" if shown else TRIPWIRE_LABELS[kind])

    return (len(fired) > 0), fired, reasons
