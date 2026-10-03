"""Step 2 — the verdict card (terminal render)."""

from __future__ import annotations

from typing import TYPE_CHECKING

from .types import Verdict

if TYPE_CHECKING:
    from .fanout import FanoutItem

_GREEN = "\033[32m"
_RED = "\033[31m"
_DIM = "\033[2m"
_BOLD = "\033[1m"
_RESET = "\033[0m"

_YELLOW = "\033[33m"
_STATUS_COLOR = {"SAFE": _GREEN, "BLOCK": _RED, "ERROR": _YELLOW}


def render_terminal(v: Verdict, color: bool = True) -> str:
    W = 64
    inner = W - 2          # columns between the vertical bars
    text_w = inner - 2     # usable text width (1-space margin each side)

    band = _RED if v.verdict == "BLOCK" else (_GREEN if v.verdict == "SAFE" else "")
    paint = (lambda s: f"{band}{s}{_RESET}") if (color and band) else (lambda s: s)

    def line(text: str = "") -> str:
        # Paint only the side bars so the box glows red/green all the way down; the inner
        # text stays default-colour for readability.
        return paint("│") + " " + text[:text_w].ljust(text_w) + " " + paint("│")

    def rule(left: str, right: str) -> str:
        return left + "─" * inner + right

    out = [paint(rule("┌", "┐")),
           paint("│ " + f"AIRLOCK — {v.verdict}".ljust(text_w) + " │"),
           paint(rule("├", "┤")),
           line("package: " + " ".join(v.package.split())),
           line(),
           line("why:")]

    reasons = v.reasons or (["No suspicious behaviour observed."] if v.verdict == "SAFE" else ["—"])
    for r in reasons:
        for i, chunk in enumerate(_wrap(r, text_w - 4)):
            out.append(line(("  • " if i == 0 else "    ") + chunk))

    if v.static and v.static.available:
        out.append(line())
        out.append(line(f"static read: {v.static.score}/10"))

    if v.similarity and v.similarity.available:
        out.append(line())
        pct = round(v.similarity.score * 100)
        flag = "  ⚠ strong match" if v.similarity.strong else ""
        out.append(line(f"known-malware match: {v.similarity.family} ({pct}%){flag}"))

    if v.reputation and v.reputation.available and v.reputation.notes:
        out.append(line())
        out.append(line("reputation:"))
        for note in v.reputation.notes:
            for i, chunk in enumerate(_wrap(note, text_w - 4)):
                out.append(line(("  - " if i == 0 else "    ") + chunk))

    out.append(paint(rule("└", "┘")))
    return "\n".join(out)


def _row_reason(item: "FanoutItem") -> str:
    """A single, whitespace-collapsed reason line for one package in the grid."""
    if item.error:
        return " ".join(item.error.split())
    v = item.verdict
    if v is None:
        return "…"
    reason = v.reasons[0] if v.reasons else (
        "No suspicious behaviour observed." if v.verdict == "SAFE" else "—")
    return " ".join(reason.split())


def render_grid(items: "list[FanoutItem]", color: bool = True) -> str:
    """A compact one-row-per-package grid for the requirements.txt fan-out (§10 step 5)."""
    W = 64
    inner = W - 2
    text_w = inner - 2

    n = len(items)
    blocked = sum(1 for i in items if i.label == "BLOCK")
    errored = sum(1 for i in items if i.label == "ERROR")
    safe = n - blocked - errored

    def rule(left: str, right: str) -> str:
        return left + "─" * inner + right

    def line(text: str = "", pad: int = 0) -> str:
        # `pad` = count of zero-width colour codes in `text` that shouldn't eat column width.
        return "│ " + text.ljust(text_w + pad) + " │"

    def paint(status: str, s: str) -> tuple[str, int]:
        band = _STATUS_COLOR.get(status, "")
        if color and band:
            return f"{band}{s}{_RESET}", len(band) + len(_RESET)
        return s, 0

    head = f"AIRLOCK — fan-out: {n} package{'s' if n != 1 else ''}"
    head += f" · {blocked} BLOCK · {safe} SAFE"
    if errored:
        head += f" · {errored} ERROR"

    out = [rule("┌", "┐"),
           "│ " + head[:text_w].ljust(text_w) + " │",
           rule("├", "┤")]

    status_w = 5                                   # SAFE / BLOCK / ERROR all ≤ 5 chars
    name_w = min(max((len(i.spec) for i in items), default=0), 20)
    reason_w = max(text_w - status_w - 1 - name_w - 1, 8)

    for i in items:
        status = i.label
        name = i.spec if len(i.spec) <= name_w else i.spec[: name_w - 1] + "…"
        reason = _row_reason(i)
        if len(reason) > reason_w:
            reason = reason[: reason_w - 1] + "…"
        status_cell, pad = paint(status, status.ljust(status_w))
        out.append(line(f"{status_cell} {name.ljust(name_w)} {reason}", pad=pad))

    out.append(rule("└", "┘"))
    return "\n".join(out)


def _wrap(text: str, width: int) -> list[str]:
    words = text.split()
    out, cur = [], ""
    for w in words:
        if len(cur) + len(w) + 1 > width:
            if cur:
                out.append(cur)
            cur = w[:width]
        else:
            cur = f"{cur} {w}".strip()
    if cur:
        out.append(cur)
    return out or [""]
