"""Shared data types — the contract between the gate and its callers (e.g. the hook)."""

from __future__ import annotations

from dataclasses import dataclass, field, asdict
from typing import Any, Literal

VerdictLabel = Literal["SAFE", "BLOCK", "ERROR"]


@dataclass
class Event:
    """One line of the behaviour trace captured inside the sandbox."""

    kind: str          # read_secret | connect | spawn_shell
    detail: str

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class StaticReport:
    """Nosana's static read of the source (step 3)."""

    score: int             # 0-10 suspicion
    summary: str
    available: bool = True

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class Reputation:
    """Reputation signal: package age, downloads, typosquat distance, known CVEs, + optional Oxylabs web intel."""

    package: str
    exists_on_pypi: bool = False
    age_days: int | None = None
    downloads_last_month: int | None = None
    nearest_popular: str | None = None
    typosquat: bool = False
    osv_ids: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    available: bool = True

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class MalwareMatch:
    """Doubleword embeddings similarity: the closest known-malware pattern to this source."""

    family: str            # e.g. "aws-credential-stealer" — the nearest corpus family
    score: float           # 0.0-1.0 cosine similarity to that family's sample
    strong: bool           # score >= the flag threshold (a real look-alike)
    available: bool = True

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class Verdict:
    """The result of check(package). This is the contract the hook consumes."""

    package: str
    verdict: VerdictLabel
    reasons: list[str] = field(default_factory=list)
    tripwires: list[str] = field(default_factory=list)
    events: list[Event] = field(default_factory=list)
    static: StaticReport | None = None
    reputation: Reputation | None = None
    similarity: MalwareMatch | None = None    # Doubleword — closest known-malware match
    judge_raw: dict[str, Any] | None = None
    timings: dict[str, float] = field(default_factory=dict)   # per-stage seconds: run/read/match/judge/total
    error: str | None = None

    @property
    def blocked(self) -> bool:
        return self.verdict == "BLOCK"

    def to_dict(self) -> dict[str, Any]:
        d = asdict(self)
        return d
