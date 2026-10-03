"""Oxylabs web intel — the 'senses' leg (perception & data).

Queries Oxylabs' Web Scraper API for live web signals about a package *name*: is it being
reported as malware / a typosquat anywhere on the public web? This catches brand-new
malware that the fixed free advisory DBs (OSV) haven't catalogued yet — the exact gap a
static advisory list can't cover.

Optional and fail-safe: with no OXYLABS_USERNAME / OXYLABS_PASSWORD set, or on any error /
timeout, returns [] and the free PyPI/OSV signals (reputation.py) carry reputation alone.
Runs on the orchestrator (full internet), never inside the sandbox (§8).

Concurrency: the fan-out fires one of these per package across the thread pool. The short
timeout + degrade-on-error keep a slow or blocked scrape from stalling the grid.
"""

from __future__ import annotations

import base64
import json
import urllib.request

from ..config import Config

_ENDPOINT = "https://realtime.oxylabs.io/v1/queries"
_TIMEOUT = 15.0  # scraping is slower than a plain API, but short enough not to stall a fan-out

# Terms that, appearing alongside the package name, suggest the web is flagging it.
_RED_TERMS = (
    "malware", "malicious", "typosquat", "supply chain", "stealer", "credential",
    "backdoor", "compromis", "trojan", "exfiltrat", "phishing", "cryptominer",
)


def web_intel(package: str, config: Config) -> list[str]:
    """Return web-intel notes about `package` (empty if unconfigured or on any failure)."""
    user, pw = config.oxylabs_username, config.oxylabs_password
    if not (user and pw):
        return []

    query = f'"{package}" pypi package malware OR malicious OR typosquat'
    body = json.dumps({"source": "google_search", "query": query, "parse": True}).encode()
    auth = base64.b64encode(f"{user}:{pw}".encode()).decode()
    req = urllib.request.Request(
        _ENDPOINT, method="POST", data=body,
        headers={"Authorization": f"Basic {auth}", "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=_TIMEOUT) as resp:  # noqa: S310 (fixed trusted host)
            data = json.loads(resp.read().decode("utf-8", "replace"))
    except Exception:
        return []
    return _summarize(package, data)


def _summarize(package: str, data: dict) -> list[str]:
    pkg = package.lower()
    hits: list[str] = []
    for r in _organic_results(data)[:10]:
        blob = f"{r.get('title', '')} {r.get('desc') or r.get('description', '')}".lower()
        if pkg in blob and any(t in blob for t in _RED_TERMS):
            title = (r.get("title") or "").strip()
            if title:
                hits.append(title)
    if not hits:
        return []
    return [
        f"web reports flag '{package}' in a malware/typosquat context "
        f"({len(hits)} of the top results, e.g. \"{hits[0][:120]}\")"
    ]


def _organic_results(data: dict) -> list[dict]:
    """Dig the organic results out of Oxylabs' parsed google_search response, defensively."""
    try:
        for result in data.get("results", []):
            content = result.get("content")
            if isinstance(content, dict):
                organic = content.get("results", {}).get("organic")
                if isinstance(organic, list):
                    return organic
    except Exception:
        pass
    return []
