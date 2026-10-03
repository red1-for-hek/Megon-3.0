"""Step 3 — READ it: a code model statically reads the package source and returns a
suspicion score. Covers the dynamic blind spot (dormant code that never ran).

Backend: prefers Nosana (the required-sponsor path). If NOSANA_ENDPOINT is unset, falls
back to ai& reading the source itself — a separate call that only sees the source (still
blind to the behaviour trace), so the independence between reader and judge is preserved.
If neither is configured, returns an unavailable report and the gate runs on tripwires
(+ ai& judge on the trace) alone.
"""

from __future__ import annotations

import json
import os
import re
import tarfile
import tempfile
import urllib.request
import zipfile

from ..config import Config
from .llm import chat
from ..types import StaticReport

MAX_SOURCE_CHARS = 16_000  # ~4k tokens — fits the ai& fallback context with room to spare

PROMPT = (
    "You are a security analyst. Below is the source of a Python package. "
    "Look for malicious behaviour: credential/secret theft, data exfiltration, "
    "reverse shells, command-and-control, obfuscated or dynamically-executed payloads, "
    "install-time side effects. Respond with ONLY a JSON object: "
    '{"score": <integer 0-10 suspicion>, "summary": "<one or two sentences>"}.\n\n'
    "SOURCE:\n"
)


def _fetch_source_text(package: str) -> str | None:
    """Fetch `package`'s source and concatenate its Python.

    Local path → read straight off disk. Otherwise download the sdist *directly* from PyPI —
    NOT via `pip download --no-binary :all:`, which forces a source build for C packages
    (Pillow, numpy, …) and hangs for the whole timeout, returning nothing.
    """
    # Local path? read it directly.
    if os.path.exists(package):
        return _read_py_tree(package)

    url = _source_url(package)
    if not url:
        return None
    dest = tempfile.mkdtemp(prefix="airlock_src_")
    archive = os.path.join(dest, os.path.basename(url.split("#", 1)[0]))
    extract_to = os.path.join(dest, "x")
    try:
        urllib.request.urlretrieve(url, archive)  # noqa: S310 (PyPI-hosted URL)
        if archive.endswith((".tar.gz", ".tgz")):
            with tarfile.open(archive) as t:
                t.extractall(extract_to)  # noqa: S202 (throwaway dir)
        elif archive.endswith((".zip", ".whl")):
            with zipfile.ZipFile(archive) as z:
                z.extractall(extract_to)
        else:
            return None
        return _read_py_tree(extract_to)
    except Exception:
        return None


def _source_url(package: str) -> str | None:
    """Newest source URL for `package` from the PyPI JSON API — the sdist (has setup.py, so
    install-time payloads are visible), or a wheel as a fallback (still carries the .py)."""
    name = re.split(r"[\[<>=!~; ]", package, maxsplit=1)[0].strip()
    try:
        with urllib.request.urlopen(f"https://pypi.org/pypi/{name}/json", timeout=15) as r:  # noqa: S310
            data = json.loads(r.read().decode("utf-8", "replace"))
    except Exception:
        return None
    files = data.get("urls", [])
    for want in ("sdist", "bdist_wheel"):
        for f in files:
            if f.get("packagetype") == want and f.get("url"):
                return f["url"]
    return None


def _read_py_tree(root: str) -> str | None:
    chunks: list[str] = []
    total = 0
    for dirpath, _dirs, files in os.walk(root):
        for fn in files:
            if fn.endswith((".py", ".cfg", ".toml")) or fn == "setup.py":
                fp = os.path.join(dirpath, fn)
                try:
                    with open(fp, encoding="utf-8", errors="replace") as f:
                        text = f.read()
                except Exception:
                    continue
                chunks.append(f"# ---- {os.path.relpath(fp, root)} ----\n{text}")
                total += len(text)
                if total > MAX_SOURCE_CHARS:
                    break
        if total > MAX_SOURCE_CHARS:
            break
    if not chunks:
        return None
    return "\n\n".join(chunks)[:MAX_SOURCE_CHARS]


def static_read(package: str, config: Config, *, reader: str = "auto") -> StaticReport:
    """Read the source with the selected backend.

    reader: "auto" (Nosana, ai& fallback), "nosana" (Nosana only), or "k" (Kimi).
    """
    use_nosana = bool(config.nosana_endpoint)
    use_aiand = bool(config.aiand_api_key)
    if reader == "k":
        if not config.kimi_api_key:
            return StaticReport(score=0, available=False,
                                summary="Kimi reader (k) selected but KIMI_API_KEY is not set.")
    elif reader == "nosana":
        if not use_nosana:
            return StaticReport(score=0, available=False,
                                summary="Nosana reader selected but NOSANA_ENDPOINT is not set.")
    elif not (use_nosana or use_aiand):
        return StaticReport(score=0, available=False,
                            summary="No static-read backend configured (set NOSANA_ENDPOINT or AIAND_API_KEY).")

    source = _fetch_source_text(package)
    if not source:
        return StaticReport(score=0, summary="Could not fetch package source; static read skipped.", available=False)

    if reader == "k":
        return _analyze(source, config.kimi_base_url, config.kimi_api_key, config.kimi_model, "Kimi")
    if reader == "nosana":
        return _analyze(source, config.nosana_endpoint, config.nosana_api_key, config.nosana_model, "Nosana")

    if use_nosana:
        report = _analyze(source, config.nosana_endpoint, config.nosana_api_key, config.nosana_model, "Nosana")
        if report.available or not use_aiand:
            return report
        # Nosana is configured but failed (down / unreachable) — fall back to ai& so the static
        # half still runs instead of silently dropping out.
        return _analyze(source, config.aiand_base_url, config.aiand_api_key, config.aiand_model, "ai& (Nosana fallback)")
    return _analyze(source, config.aiand_base_url, config.aiand_api_key, config.aiand_model, "ai& (Nosana fallback)")


def _analyze(source: str, base_url: str, api_key: str, model: str, via: str) -> StaticReport:
    try:
        resp = chat(base_url, api_key, model, [{"role": "user", "content": PROMPT + source}], json_mode=True)
        content = (resp.choices[0].message.content or "").strip()
        data = _parse_json(content)
        score = max(0, min(10, int(data.get("score", 0))))
        summary = str(data.get("summary", "")).strip() or "No summary returned."
        return StaticReport(score=score, summary=f"[{via}] {summary}", available=True)
    except Exception as e:  # network/model/parse failure — degrade, don't crash the gate
        return StaticReport(score=0, summary=f"Static read via {via} failed: {e}", available=False)


def _parse_json(text: str) -> dict:
    try:
        return json.loads(text)
    except Exception:
        start, end = text.find("{"), text.rfind("}")
        if 0 <= start < end:
            try:
                return json.loads(text[start:end + 1])
            except Exception:
                pass
    return {}
