"""Sponsor integration registry + live self-check.

Single source of truth for which sponsored products Airlock integrates, where each is
wired, and a real ping so anyone — you before a demo, or a reviewer auditing the repo —
can confirm every integration is actually reachable, not just present in the code:

    python -m airlock --sponsors

Each check does the *minimum real API call* that proves the credential works (Daytona:
list sandboxes; ai&/Nosana: a one-token chat; Oxylabs: a tiny scrape query). It never
prints secrets, and degrades to a clear status line instead of raising. Statuses:

    ok    — the credential is set and the endpoint answered
    fail  — configured, but the call errored (bad key / down / unreachable)
    skip  — not configured (Airlock degrades gracefully; see each row's note)

The rows here mirror the real call sites, so this file also *is* the machine-readable
map of the integration (see SPONSORS.md for the prose version).
"""

from __future__ import annotations

import base64
import json
import time
import urllib.request
from dataclasses import dataclass

from .config import Config, load_config

OK, FAIL, SKIP = "ok", "fail", "skip"


@dataclass
class Result:
    name: str       # "Daytona"
    role: str       # short description of the job it does in Airlock
    module: str     # the file where it's wired (repo-relative)
    status: str     # ok | fail | skip
    detail: str     # human-readable outcome — never a secret
    elapsed: float = 0.0


def _ping_chat(base_url: str, api_key: str, model: str, timeout: float = 20.0) -> str:
    """Smallest possible OpenAI-compatible completion — proves the endpoint answers."""
    from openai import OpenAI, BadRequestError

    client = OpenAI(api_key=api_key, base_url=base_url, timeout=timeout, max_retries=0)
    msgs = [{"role": "user", "content": "ping"}]
    try:
        client.chat.completions.create(model=model, messages=msgs, max_tokens=5)
    except BadRequestError:
        # Some models reject max_tokens (or want max_completion_tokens); a plain call still proves auth.
        client.chat.completions.create(model=model, messages=msgs)
    return model


def _check_daytona(c: Config) -> tuple[str, str]:
    if not c.daytona_api_key:
        return SKIP, "DAYTONA_API_KEY not set — the gate can't run without it"
    from daytona import Daytona, DaytonaConfig

    kwargs = {"api_key": c.daytona_api_key}
    if c.daytona_api_url:
        kwargs["api_url"] = c.daytona_api_url
    sandboxes = list(Daytona(DaytonaConfig(**kwargs)).list())  # authenticated, creates nothing
    return OK, f"authenticated — {len(sandboxes)} sandbox(es) visible"


def _check_aiand(c: Config) -> tuple[str, str]:
    if not c.aiand_api_key:
        return SKIP, "AIAND_API_KEY not set — gate falls back to the plain-code tripwire floor"
    return OK, f"{_ping_chat(c.aiand_base_url, c.aiand_api_key, c.aiand_model)} responded"


def _check_nosana(c: Config) -> tuple[str, str]:
    if not c.nosana_endpoint:
        return SKIP, "NOSANA_ENDPOINT not set — ai& does the static read as a fallback"
    return OK, f"{_ping_chat(c.nosana_endpoint, c.nosana_api_key, c.nosana_model)} responded"


def _check_doubleword(c: Config) -> tuple[str, str]:
    if not c.doubleword_api_key:
        return SKIP, "DOUBLEWORD_API_KEY not set — no known-malware similarity match"
    from openai import OpenAI

    client = OpenAI(api_key=c.doubleword_api_key, base_url=c.doubleword_base_url,
                    timeout=20, max_retries=0)
    resp = client.embeddings.create(model=c.doubleword_embed_model, input=["ping"])
    dims = len(resp.data[0].embedding)
    return OK, f"{c.doubleword_embed_model} responded ({dims}-dim embedding)"


def _check_oxylabs(c: Config) -> tuple[str, str]:
    if not (c.oxylabs_username and c.oxylabs_password):
        return SKIP, "OXYLABS_USERNAME/PASSWORD not set — reputation uses free PyPI/OSV APIs alone"
    auth = base64.b64encode(f"{c.oxylabs_username}:{c.oxylabs_password}".encode()).decode()
    body = json.dumps({"source": "google_search", "query": "airlock sponsor self-check"}).encode()
    req = urllib.request.Request(
        "https://realtime.oxylabs.io/v1/queries", method="POST", data=body,
        headers={"Authorization": f"Basic {auth}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=20) as resp:  # noqa: S310 (fixed trusted host)
        code = resp.getcode()
    return OK, f"Web Scraper API authenticated (HTTP {code})"


# The registry — each row names the sponsor, its job, the file it's wired in, and its check.
_CHECKS = [
    ("Daytona", "runs it — disposable sandbox execution + behaviour trace",
     "airlock/sandbox/detonate.py", _check_daytona),
    ("Nosana", "reads it — static source analysis on a GPU",
     "airlock/analysis/static_read.py", _check_nosana),
    ("Doubleword", "matches it — embeddings similarity vs known malware",
     "airlock/analysis/doubleword.py", _check_doubleword),
    ("ai&", "judges it — weighs the evidence into SAFE/BLOCK",
     "airlock/analysis/judge.py", _check_aiand),
    ("Oxylabs", "investigates it — live web reputation / threat intel",
     "airlock/analysis/oxylabs.py", _check_oxylabs),
]


def run_checks(config: Config | None = None) -> list[Result]:
    config = config or load_config()
    results: list[Result] = []
    for name, role, module, fn in _CHECKS:
        t0 = time.monotonic()
        try:
            status, detail = fn(config)
        except Exception as e:  # a live failure is a fail, never a crash of the checker
            status, detail = FAIL, str(e).splitlines()[0][:160]
        results.append(Result(name, role, module, status, detail, time.monotonic() - t0))
    return results


_MARK = {OK: "✓", FAIL: "✗", SKIP: "–"}
_COLOR = {OK: "\033[32m", FAIL: "\033[31m", SKIP: "\033[2m"}
_RESET = "\033[0m"


def format_report(results: list[Result], color: bool = True) -> str:
    def paint(status: str, text: str) -> str:
        return f"{_COLOR[status]}{text}{_RESET}" if color else text

    lines = ["Airlock — sponsor integration self-check", ""]
    for r in results:
        mark = paint(r.status, f"{_MARK[r.status]} {r.name}")
        lines.append(f"  {mark:<24} {r.role}")
        lines.append(f"      {r.module}")
        lines.append(f"      {r.detail}  ({r.elapsed:.1f}s)")
        lines.append("")

    n_ok = sum(r.status == OK for r in results)
    n_fail = sum(r.status == FAIL for r in results)
    n_skip = sum(r.status == SKIP for r in results)
    summary = f"{n_ok} live · {n_fail} failing · {n_skip} not configured"
    lines.append(paint(FAIL if n_fail else OK, summary))
    if n_skip:
        lines.append("\033[2m  (skipped rows aren't errors — Airlock degrades gracefully; see the note on each.)\033[0m"
                     if color else "  (skipped rows aren't errors — Airlock degrades gracefully; see the note on each.)")
    return "\n".join(lines)


def selfcheck_exit_code(results: list[Result]) -> int:
    return 1 if any(r.status == FAIL for r in results) else 0
