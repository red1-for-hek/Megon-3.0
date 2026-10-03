"""The enforcement hook — Claude Code `PreToolUse`, step 4 of the build plan (§5).

Claude Code runs this before every `Bash` command. It reads the pending command as JSON
on stdin, and:

  * anything that isn't a `pip install` → allow instantly (a ~millisecond regex sniff; the
    heavy gate code isn't even imported on this path).
  * a `pip install <pkg>` → run each package through `airlock.check()` and

      - BLOCK  → tell Claude Code to *deny* the command, with the reason.
      - SAFE   → stay silent, letting the install proceed through the normal flow.
      - ERROR  → *ask* the human (the gate couldn't complete — e.g. Daytona is down);
                 fail to a prompt, never a silent allow.

The demo villain (`python-pillow`) isn't on PyPI — it's ours, served from a local index
(see resolve_target). A name that matches a directory in the index is detonated from there;
everything else is treated as a registry name and installs from PyPI inside the sandbox.

Register it via `.claude/settings.json` (a PreToolUse hook matching `Bash`). Run standalone:
    echo '{"tool_name":"Bash","tool_input":{"command":"pip install python-pillow"}}' \
        | python3 -m airlock.hook
"""

from __future__ import annotations

import json
import os
import re
import shlex
import sys
from pathlib import Path

# Shell control operators that separate one simple command from the next.
_SEGMENT_SPLIT = re.compile(r"\s*(?:&&|\|\||;|\n|\|)\s*")

# A pip "driver" token: pip, pip3, pip3.12, …
_PIP_RE = re.compile(r"^pip[0-9.]*$")
_PYTHON_RE = re.compile(r"^(?:python[0-9.]*|py)$")

# Flags that consume the following token as their value (so it isn't a package name).
_VALUE_FLAGS = {
    "-r", "--requirement", "-c", "--constraint", "-t", "--target",
    "-i", "--index-url", "--extra-index-url", "-f", "--find-links",
    "--no-binary", "--only-binary", "--prefix", "--root", "--src",
    "-e", "--editable", "--platform", "--python-version", "--implementation",
    "--abi", "--progress-bar", "--report", "--cache-dir", "--log", "--proxy",
    "--retries", "--timeout", "--exists-action", "--config-settings",
}

# Default "tiny local index" (§10): a directory of local packages the hook prefers over
# PyPI. Resolves relative to this file so it works regardless of the agent's cwd.
_DEFAULT_INDEX = Path(__file__).resolve().parent.parent / "examples" / "local-index"


def _local_index() -> Path:
    return Path(os.environ.get("AIRLOCK_LOCAL_INDEX") or _DEFAULT_INDEX)


def _looks_like_path(spec: str) -> bool:
    return (
        "/" in spec
        or os.sep in spec
        or spec.endswith((".whl", ".tar.gz", ".zip"))
        or os.path.exists(spec)
    )


def _bare_name(spec: str) -> str:
    """Strip extras and version constraints from a requirement spec → the package name."""
    return re.split(r"[\[<>=!~;@ ]", spec, maxsplit=1)[0].strip().strip("'\"")


def _normalize(name: str) -> str:
    """PEP 503 normalization for case/separator-insensitive index lookup."""
    return re.sub(r"[-_.]+", "-", name).lower()


def extract_pip_packages(command: str) -> list[str]:
    """Return the package specs from any `pip install` in `command` (empty if none).

    Handles `pip install`, `pip3 install`, `python -m pip install`, `uv pip install`,
    flags, version pins, and compound commands (`cd x && pip install y`). A bare
    `pip install -r requirements.txt` yields no names — nothing for the hook to check.
    """
    packages: list[str] = []
    for segment in _SEGMENT_SPLIT.split(command):
        segment = segment.strip()
        if not segment or "install" not in segment:
            continue
        try:
            tokens = shlex.split(segment)
        except ValueError:
            tokens = segment.split()

        install_at = _find_install(tokens)
        if install_at is None:
            continue

        i = install_at + 1
        while i < len(tokens):
            tok = tokens[i]
            if tok.startswith("-"):
                flag = tok.split("=", 1)[0]
                if flag in _VALUE_FLAGS and "=" not in tok:
                    i += 1  # skip this flag's value token too
                i += 1
                continue
            packages.append(tok)
            i += 1
    return packages


def _find_install(tokens: list[str]) -> int | None:
    """Index of the `install` subcommand if these tokens are a pip install, else None."""
    n = len(tokens)
    for i, tok in enumerate(tokens):
        base = os.path.basename(tok)
        # pip … install
        if _PIP_RE.match(base):
            j = i + 1
            while j < n and tokens[j].startswith("-"):
                j += 1
            if j < n and tokens[j] == "install":
                return j
        # python -m pip install  /  uv pip install
        if (_PYTHON_RE.match(base) and i + 2 < n and tokens[i + 1] == "-m"
                and tokens[i + 2] == "pip"):
            if i + 3 < n and tokens[i + 3] == "install":
                return i + 3
        if base == "uv" and i + 2 < n and tokens[i + 1] == "pip" and tokens[i + 2] == "install":
            return i + 2
    return None


def resolve_target(spec: str) -> tuple[str, bool]:
    """Map a pip spec to a detonation target. Returns (target, is_local).

    A local path is passed through untouched. A registry name is looked up in the local
    index — if a matching package directory exists there it's detonated locally (the demo
    villain path); otherwise the bare name is returned to install from PyPI.
    """
    if _looks_like_path(spec):
        return spec, True
    name = _bare_name(spec)
    index = _local_index()
    if index.is_dir():
        wanted = _normalize(name)
        for child in index.iterdir():
            if child.is_dir() and _normalize(child.name) == wanted:
                return str(child), True
    return name, False


# --- Claude Code hook I/O ---------------------------------------------------------------

def _emit(decision: str, reason: str) -> None:
    """Print a PreToolUse permission decision ('deny' | 'ask') for Claude Code."""
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": decision,
            "permissionDecisionReason": reason,
        }
    }))


def _wrap_bullet(text: str, width: int = 62) -> list[str]:
    """A '• ' bullet, wrapped with a hanging indent so it reads cleanly in a terminal."""
    words, rows, cur = text.split(), [], ""
    for w in words:
        if cur and len(cur) + 1 + len(w) > width:
            rows.append(cur)
            cur = w
        else:
            cur = f"{cur} {w}".strip()
    if cur:
        rows.append(cur)
    if not rows:
        return []
    return ["    • " + rows[0]] + ["      " + r for r in rows[1:]]


def _clean_summary(summary: str | None) -> str:
    """Drop the analyzer's [backend] tag and trim overly long model output."""
    s = (summary or "").strip()
    for tag in ("[Nosana]", "[ai& (Nosana fallback)]", "[ai&]"):
        if s.startswith(tag):
            s = s[len(tag):].strip()
            break
    if len(s) > 200:
        s = s[:199].rsplit(" ", 1)[0] + "…"
    return s


def _block_reason(pkg: str, verdict) -> str:
    """A plain-English explanation of the block, grouped the way it was checked:
    what it did when run, what its code says, and what's known about the package.
    No jargon — written so a non-specialist immediately gets why it's dangerous.
    """
    tw_n = len(verdict.tripwires)
    tw_reasons = verdict.reasons[:tw_n]
    judge_reasons = verdict.reasons[tw_n:]

    out = [
        f"🚫  BLOCKED: {pkg} is not safe to install",
        "",
        "Airlock installed this package on a throwaway machine and watched",
        "exactly what it did. Here's what it caught:",
    ]

    # What it did when we ran it (the deterministic sandbox tripwires).
    if tw_reasons:
        out += ["", "  When we ran it"]
        for r in tw_reasons:
            out += _wrap_bullet(r)
    else:
        out += ["", "  When we ran it"]
        out += _wrap_bullet("Nothing obvious during the short run, but that alone does not make it safe")

    # What its code says (static analysis).
    if verdict.static and verdict.static.available:
        summary = _clean_summary(verdict.static.summary)
        if summary:
            out += ["", f"  When we read its code  (rated {verdict.static.score}/10 for risk)"]
            out += _wrap_bullet(summary)

    # What's known about the package (reputation).
    if verdict.reputation and verdict.reputation.available and verdict.reputation.notes:
        out += ["", "  About the package itself"]
        for note in verdict.reputation.notes:
            out += _wrap_bullet(note[:1].upper() + note[1:])

    # Our overall judgement — only when the model (not a tripwire) is what drove the block.
    if judge_reasons and not tw_reasons:
        out += ["", "  Our judgement"]
        out += _wrap_bullet("; ".join(judge_reasons[:3]))

    out += ["", "The install was stopped before it could run on your real machine."]
    return "\n".join(out)


def _evaluate(specs: list[str]) -> tuple[str | None, str]:
    """Run each install spec through the gate. Returns (decision, reason).

    May raise if the gate can't even be imported/run — the caller turns that into 'ask'
    (an install we couldn't evaluate is never silently allowed).
    """
    from .gate import check  # heavy imports deferred off the fast path

    errors: list[str] = []
    for spec in specs:
        target, _is_local = resolve_target(spec)
        try:
            verdict = check(target)
        except Exception as e:  # one package erroring shouldn't hide a block on another
            errors.append(f"{spec}: {e}")
            continue
        if verdict.verdict == "BLOCK":
            return "deny", _block_reason(spec, verdict)
        if verdict.verdict == "ERROR":
            errors.append(f"{spec}: {verdict.error or 'gate error'}")

    if errors:
        detail = "; ".join(errors)
        return "ask", (f"⚠️ Airlock could not complete its safety check "
                       f"({detail}). Approve only if you trust this install.")
    return None, ""  # all SAFE


def decide(command: str) -> tuple[str | None, str]:
    """Core decision for a Bash command. Returns (decision, reason).

    decision is None to stay silent (allow / normal flow), or 'deny' / 'ask'. An install
    the gate cannot evaluate becomes 'ask' — never a silent allow.
    """
    specs = extract_pip_packages(command)
    if not specs:
        return None, ""  # not an install → allow instantly
    try:
        return _evaluate(specs)
    except Exception as e:
        return "ask", (f"⚠️ Airlock's safety check failed to run ({e}). "
                       f"Approve only if you trust this install.")


def main() -> int:
    try:
        raw = sys.stdin.read()
        payload = json.loads(raw) if raw.strip() else {}
    except Exception:
        return 0  # can't parse input → don't get in the agent's way

    if payload.get("tool_name") != "Bash":
        return 0
    command = (payload.get("tool_input") or {}).get("command")
    if not isinstance(command, str) or not command.strip():
        return 0

    decision, reason = decide(command)
    if decision is not None:
        _emit(decision, reason)
    return 0


if __name__ == "__main__":
    sys.exit(main())
