"""Thin wrapper around OpenAI-compatible chat completions that adapts to per-model
constraints — some models reject `temperature=0` (only allow 1) or `response_format`.

It tries the most-specific request first (json + temperature 0) and falls back to
plainer variants, remembering which variant worked per model so later calls go
straight to it.

Only *capability* errors (a 400 — the model rejecting a parameter) cause a fall-back to
the next variant. Rate-limit / overload / timeout / connection errors fail fast: cycling
variants can't help, and retrying with backoff is exactly what makes a busy judge endpoint
look frozen. The gate then degrades to the tripwire verdict instead of hanging for minutes.
"""

from __future__ import annotations

from openai import OpenAI, BadRequestError

_BEST: dict[str, int] = {}  # model -> index of the request variant that worked

# Fail fast so an overloaded endpoint degrades in ~a second, not minutes.
_TIMEOUT = 45.0     # per request; a real judge call on a big prompt still fits
_MAX_RETRIES = 0    # no backoff-retry storms on 429


def chat(base_url: str, api_key: str, model: str, messages: list[dict], *, json_mode: bool = False):
    client = OpenAI(api_key=api_key, base_url=base_url,
                    timeout=_TIMEOUT, max_retries=_MAX_RETRIES)
    common = {"model": model, "messages": messages}

    variants: list[dict] = []
    if json_mode:
        variants.append({**common, "response_format": {"type": "json_object"}, "temperature": 0})
        variants.append({**common, "response_format": {"type": "json_object"}})
        variants.append({**common, "temperature": 0})
        variants.append(dict(common))
    else:
        variants.append({**common, "temperature": 0})
        variants.append(dict(common))

    order = list(range(len(variants)))
    if model in _BEST:  # try the known-good variant first
        b = _BEST[model]
        order = [b] + [i for i in order if i != b]

    last_err: Exception | None = None
    for i in order:
        try:
            resp = client.chat.completions.create(**variants[i])
            _BEST[model] = i
            return resp
        except BadRequestError as e:
            # A rejected parameter (temperature / response_format) — the next variant may work.
            last_err = e
        # Rate-limit / overload / timeout / connection errors propagate immediately: trying
        # other variants would only stack more waiting. Let the caller degrade gracefully.
    raise last_err  # type: ignore[misc]
