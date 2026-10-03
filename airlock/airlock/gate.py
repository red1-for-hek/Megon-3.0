"""The gate: check(package) → Verdict.

Ties the three steps together and applies the monotonic combine:
  final = tripwire-BLOCK OR judge-BLOCK
so the model layer can only make the gate stricter, never overrule a tripwire.

This is the contract the hook (step 4) consumes: `check(package).blocked`.
"""

from __future__ import annotations

import time
from typing import Callable

from .analysis.doubleword import similarity_match
from .analysis.judge import judge as run_judge
from .analysis.reputation import reputation as run_reputation
from .analysis.static_read import static_read
from .config import Config, load_config
from .sandbox.detonate import detonate
from .sandbox.tripwires import evaluate_tripwires
from .types import Verdict


def check(
    package: str,
    *,
    config: Config | None = None,
    mode: str = "package",
    do_static: bool = True,
    do_judge: bool = True,
    do_reputation: bool = True,
    reader: str = "auto",
    on_progress: Callable[[str], None] | None = None,
) -> Verdict:
    """Detonate, read, and judge `package`; return a Verdict (SAFE / BLOCK / ERROR).

    on_progress, if given, is called with a short human string before each slow step —
    so a long-running check shows life instead of looking frozen.
    """
    config = config or load_config()
    say = on_progress or (lambda _msg: None)

    # Step 1 — RUN it.
    say("detonating in a Daytona sandbox…")
    t_start = time.monotonic()
    timings: dict[str, float] = {}
    try:
        det = detonate(package, config, mode=mode, on_progress=say)
    except Exception as e:
        return Verdict(package=package, verdict="ERROR", error=str(e),
                       reasons=[f"detonation failed: {e}"])
    timings["run"] = time.monotonic() - t_start

    events = det.events
    tw_blocked, fired, tw_reasons = evaluate_tripwires(events)

    # Step 3 — READ it (Nosana / ai& fallback). Only meaningful for real packages.
    static = None
    if do_static and mode == "package":
        say("reading the source (static analysis)…")
        t0 = time.monotonic()
        static = static_read(package, config, reader=reader)
        timings["read"] = time.monotonic() - t0

    # MATCH it (Doubleword) — embeddings similarity to known-malware patterns.
    similarity = None
    if mode == "package" and config.doubleword_api_key:
        say("matching against known malware (Doubleword)…")
        t0 = time.monotonic()
        similarity = similarity_match(package, config)
        timings["match"] = time.monotonic() - t0

    # Reputation — free public signals (age / downloads / typosquat / OSV).
    rep = None
    if do_reputation and mode == "package":
        say("checking reputation (PyPI / OSV)…")
        t0 = time.monotonic()
        rep = run_reputation(package, config)
        timings["reputation"] = time.monotonic() - t0

    # Step 2 — JUDGE it (ai&).
    judge_out = None
    if do_judge:
        say("judging the evidence (ai&)…")
        t0 = time.monotonic()
        judge_out = run_judge(package, events, static, config,
                              tripwire_fired=tw_blocked, reputation=rep, similarity=similarity)
        timings["judge"] = time.monotonic() - t0
    timings["total"] = time.monotonic() - t_start

    # Monotonic combine: tripwire-BLOCK OR judge-BLOCK.
    reasons = list(tw_reasons)
    judge_block = bool(judge_out and judge_out.get("verdict") == "BLOCK")
    if judge_block:
        for r in judge_out.get("reasons", []):
            if r not in reasons:
                reasons.append(r)
    elif judge_out and judge_out.get("verdict") == "SAFE" and not tw_blocked:
        reasons.extend(r for r in judge_out.get("reasons", []) if r not in reasons)
    # judge ERROR / unavailable → fall back to tripwires only (graceful degrade).

    blocked = tw_blocked or judge_block
    verdict = "BLOCK" if blocked else "SAFE"
    if not reasons and verdict == "SAFE":
        reasons = ["No suspicious behaviour observed."]

    return Verdict(
        package=package,
        verdict=verdict,
        reasons=reasons,
        tripwires=fired,
        events=events,
        static=static,
        reputation=rep,
        similarity=similarity,
        judge_raw=judge_out,
        timings=timings,
    )
