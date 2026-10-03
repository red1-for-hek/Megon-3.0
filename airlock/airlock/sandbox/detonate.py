"""Step 1 — RUN it: create a Daytona sandbox, detonate the target under instrumentation,
pull the behaviour trace, delete the sandbox."""

from __future__ import annotations

import base64
import io
import json
import os
import tarfile
import time
from dataclasses import dataclass, field
from typing import Any, Callable

from daytona import Daytona, DaytonaConfig, CreateSandboxFromSnapshotParams

from ..config import Config
from .runner import build_runner, RESULT_MARKER
from ..types import Event


def _tar_b64(path: str) -> str:
    """Pack a local package (dir or file) into a base64 gzip tarball to ship into the sandbox."""
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as tar:
        tar.add(path, arcname=".")
    return base64.b64encode(buf.getvalue()).decode()


@dataclass
class Detonation:
    events: list[Event] = field(default_factory=list)
    steps: list[dict[str, Any]] = field(default_factory=list)
    raw_stdout: str = ""


def _parse_result(stdout: str) -> tuple[list[Event], list[dict[str, Any]]]:
    events: list[Event] = []
    steps: list[dict[str, Any]] = []
    for line in stdout.splitlines():
        if line.startswith(RESULT_MARKER):
            try:
                data = json.loads(line[len(RESULT_MARKER):])
            except Exception:
                continue
            events = [
                Event(kind=str(e.get("kind", "")), detail=str(e.get("detail", "")))
                for e in data.get("events", [])
            ]
            steps = data.get("steps", [])
    return events, steps


def detonate(target: str, config: Config, *, mode: str = "package", code_timeout: int = 180,
             on_progress: Callable[[str], None] | None = None) -> Detonation:
    """Detonate `target` in a fresh disposable sandbox and return its behaviour trace."""
    if not config.daytona_api_key:
        raise RuntimeError("DAYTONA_API_KEY not set — cannot create a Daytona sandbox.")
    say = on_progress or (lambda _msg: None)

    cfg_kwargs: dict[str, Any] = {"api_key": config.daytona_api_key}
    if config.daytona_api_url:
        cfg_kwargs["api_url"] = config.daytona_api_url
    daytona = Daytona(DaytonaConfig(**cfg_kwargs))

    local_payload = _tar_b64(target) if (mode == "package" and os.path.exists(target)) else None

    sandbox = None
    try:
        t0 = time.monotonic()
        sandbox = daytona.create(CreateSandboxFromSnapshotParams(language="python"))
        say(f"📦 sandbox created in {time.monotonic() - t0:.1f}s — id {sandbox.id[:8]}")
        script = build_runner(mode, target, local_payload=local_payload)
        resp = sandbox.process.code_run(script, timeout=code_timeout)
        stdout = resp.result or ""
        events, steps = _parse_result(stdout)
        return Detonation(events=events, steps=steps, raw_stdout=stdout)
    finally:
        if sandbox is not None:
            try:
                daytona.delete(sandbox)
            except Exception:
                pass
            else:
                say("💥 sandbox destroyed — whatever ran in there died with it")
