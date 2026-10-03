"""Step 2 — JUDGE it: ai& weighs the evidence (behaviour trace + static report) and
returns a plain-English SAFE / BLOCK with reasons.

ai& never sees the package source, only the reports. It is the decision-maker for the
gray zone; it cannot overrule a tripwire (that monotonic combine happens in gate.py).
Degrades gracefully: if no ai& key, returns None and the gate falls back to tripwires.
"""

from __future__ import annotations

import json

from ..config import Config
from .llm import chat
from ..types import Event, MalwareMatch, Reputation, StaticReport

SYSTEM = (
    "You are Airlock's judge. You decide whether a software package is safe to install, "
    "based only on the evidence reports given (a dynamic behaviour trace from detonating it "
    "in a sandbox, and a static source-read score). You never see the raw source yourself. "
    "Rules: if you are unsure, choose BLOCK — a false BLOCK only costs a retry, a false SAFE "
    "costs the machine. Respond with ONLY a JSON object: "
    '{"verdict": "SAFE" | "BLOCK", "reasons": ["short reason", ...]}.'
)


def judge(
    package: str,
    events: list[Event],
    static: StaticReport | None,
    config: Config,
    *,
    tripwire_fired: bool,
    reputation: Reputation | None = None,
    similarity: MalwareMatch | None = None,
) -> dict | None:
    if not config.aiand_api_key:
        return None

    rep_summary: object = "not available"
    if reputation and reputation.available:
        rep_summary = {k: v for k, v in {
            "age_days": reputation.age_days,
            "downloads_last_month": reputation.downloads_last_month,
            "typosquat_of": reputation.nearest_popular if reputation.typosquat else None,
            "osv_advisories": reputation.osv_ids or None,
            "flags": reputation.notes or None,
        }.items() if v is not None} or "no red flags"

    evidence = {
        "package": package,
        "dynamic_trace": [{"kind": e.kind, "detail": e.detail} for e in events] or "no suspicious events observed",
        "deterministic_tripwire_fired": tripwire_fired,
        "static_read": (
            {"score_out_of_10": static.score, "summary": static.summary}
            if static and static.available else "not available"
        ),
        "reputation": rep_summary,
        "similarity_to_known_malware": (
            {"closest_family": similarity.family, "similarity": similarity.score,
             "strong_match": similarity.strong}
            if similarity and similarity.available else "not available"
        ),
    }
    user = (
        "Evidence:\n" + json.dumps(evidence, indent=2) +
        "\n\nDecide the verdict. Remember: a tripwire firing means malicious behaviour was "
        "directly observed. When unsure, BLOCK."
    )

    try:
        resp = chat(
            config.aiand_base_url, config.aiand_api_key, config.aiand_model,
            [{"role": "system", "content": SYSTEM}, {"role": "user", "content": user}],
            json_mode=True,
        )
        content = (resp.choices[0].message.content or "").strip()
        data = _parse_json(content)
        verdict = str(data.get("verdict", "")).upper()
        if verdict not in ("SAFE", "BLOCK"):
            verdict = "BLOCK"  # unparseable → fail safe
        reasons = data.get("reasons") or []
        if isinstance(reasons, str):
            reasons = [reasons]
        return {"verdict": verdict, "reasons": [str(r) for r in reasons]}
    except Exception as e:
        return {"verdict": "ERROR", "reasons": [f"ai& judge failed: {e}"]}


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
