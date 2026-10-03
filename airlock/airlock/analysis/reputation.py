"""Reputation — signals about a package.

Free public sources (no keys, no cost): PyPI JSON (age), pypistats (downloads), OSV.dev
(known vulns / malware advisories), and a local typosquat check (edit distance + affix
variants against popular package names). When Oxylabs creds are configured, adds live web
intel (oxylabs.py) — "is this name reported as malware anywhere?" — the 'senses' leg.

Feeds ai&'s evidence. Especially load-bearing for the typosquat demo: `python-pillow` is
flagged as `python-` + `pillow` (a top package). Degrades gracefully on any network error.
"""

from __future__ import annotations

import json
import os
import urllib.request
from datetime import datetime, timezone

from ..config import Config
from ..types import Reputation
from .oxylabs import web_intel

# A small set of very popular PyPI packages — enough to catch common typosquats.
POPULAR = {
    "pillow", "requests", "numpy", "pandas", "scipy", "matplotlib", "flask", "django",
    "fastapi", "boto3", "botocore", "urllib3", "certifi", "setuptools", "wheel", "pip",
    "six", "python-dateutil", "pytz", "click", "jinja2", "werkzeug", "sqlalchemy",
    "pydantic", "aiohttp", "beautifulsoup4", "lxml", "pytest", "black", "flake8", "mypy",
    "tensorflow", "torch", "scikit-learn", "keras", "openai", "anthropic", "transformers",
    "tqdm", "pyyaml", "cryptography", "redis", "celery", "gunicorn", "uvicorn", "httpx",
    "rich", "typer", "poetry", "pip-tools", "colorama", "attrs", "packaging", "idna",
    "charset-normalizer", "markupsafe", "cffi", "pyparsing", "websockets", "selenium",
}


def _get_json(url: str, *, method: str = "GET", body: bytes | None = None, timeout: float = 8.0):
    headers = {"User-Agent": "airlock/0.1"}
    if body is not None:
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, method=method, data=body, headers=headers)
    with urllib.request.urlopen(req, timeout=timeout) as resp:  # noqa: S310 (trusted hosts)
        return json.loads(resp.read().decode("utf-8", "replace"))


def _levenshtein(a: str, b: str) -> int:
    if a == b:
        return 0
    if not a:
        return len(b)
    if not b:
        return len(a)
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


def _typosquat_check(name: str) -> tuple[bool, str | None, str | None]:
    n = name.lower()
    if n in POPULAR:
        return False, None, None
    for pre in ("python-", "py-", "python_", "py"):
        if n.startswith(pre) and n[len(pre):] in POPULAR:
            base = n[len(pre):]
            return True, base, f"its name mimics the popular package '{base}'"
    for suf in ("-python", "_python", "-py", "2", "3"):
        if n.endswith(suf) and n[: -len(suf)] in POPULAR:
            base = n[: -len(suf)]
            return True, base, f"its name mimics the popular package '{base}'"
    best, best_d = None, 99
    for p in POPULAR:
        d = _levenshtein(n, p)
        if d < best_d:
            best, best_d = p, d
    if best and 1 <= best_d <= 2:
        return True, best, f"its name is only {best_d} character(s) off the popular package '{best}'"
    return False, best, None


def reputation(package: str, config: Config | None = None) -> Reputation:
    name = package.strip()
    # The demo villain is detonated by local path; reputation is about the *name*, so use the
    # package/dir basename (…/python-pillow -> python-pillow) to catch typosquats and PyPI status.
    if os.sep in name or (os.altsep and os.altsep in name) or os.path.exists(name):
        base = os.path.basename(name.rstrip("/\\")) or name
        for suf in (".tar.gz", ".tgz", ".zip", ".whl"):
            if base.endswith(suf):
                base = base[: -len(suf)]
        name = base
    notes: list[str] = []

    # PyPI JSON — existence + age (earliest upload across all releases).
    exists, age_days = False, None
    try:
        data = _get_json(f"https://pypi.org/pypi/{name}/json")
        exists = True
        times = [
            f.get("upload_time_iso_8601") or f.get("upload_time")
            for rel in data.get("releases", {}).values() for f in rel
        ]
        times = [t for t in times if t]
        if times:
            first = min(times).replace("Z", "+00:00")
            age_days = (datetime.now(timezone.utc) - datetime.fromisoformat(first)).days
    except Exception:
        exists = False

    # pypistats — recent downloads.
    downloads = None
    try:
        d = _get_json(f"https://pypistats.org/api/packages/{name.lower()}/recent")
        downloads = d.get("data", {}).get("last_month")
    except Exception:
        pass

    # local typosquat check.
    typo, nearest, typo_reason = _typosquat_check(name)

    # OSV.dev — known vulnerabilities / malware advisories.
    osv_ids: list[str] = []
    try:
        body = json.dumps({"package": {"name": name, "ecosystem": "PyPI"}}).encode()
        r = _get_json("https://api.osv.dev/v1/query", method="POST", body=body)
        osv_ids = [v.get("id") for v in r.get("vulns", []) if v.get("id")]
    except Exception:
        pass

    if not exists:
        notes.append("not a real published package (not on PyPI)")
    if age_days is not None and age_days < 30:
        notes.append(f"brand new — only {age_days} days old")
    if downloads is not None and downloads < 1000:
        notes.append(f"almost nobody uses it ({downloads} downloads last month)")
    if typo and typo_reason:
        notes.append(typo_reason)
    # Only loudly flag known-malware advisories (MAL-*), not patched historical CVEs.
    malware = [i for i in osv_ids if i.upper().startswith("MAL")]
    if malware:
        notes.append("flagged as known malware in the public advisory database: " + ", ".join(malware[:3]))

    # Oxylabs web intel (the 'senses') — optional; layered on top of the free signals above.
    if config is not None:
        notes.extend(web_intel(name, config))

    return Reputation(
        package=name, exists_on_pypi=exists, age_days=age_days,
        downloads_last_month=downloads, nearest_popular=nearest,
        typosquat=typo, osv_ids=osv_ids, notes=notes, available=True,
    )
