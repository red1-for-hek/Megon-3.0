"""Airlock — a safety gate for AI agents.

Public entry point: `airlock.check(package)` → Verdict.
See DESIGN.md for the architecture (run it / read it / judge it).
"""

__all__ = ["check", "Verdict"]


def __getattr__(name: str):
    # Lazy so importing the package (e.g. `python -m airlock.hook` on its fast path) doesn't
    # pull in Daytona / the LLM client until check() is actually used. PEP 562.
    if name == "check":
        from .gate import check
        return check
    if name == "Verdict":
        from .types import Verdict
        return Verdict
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
