"""Tests for the enforcement hook (airlock/hook.py).

Offline: the gate's check() is stubbed, so no Daytona / keys are needed. Run either with
pytest, or standalone:  python tests/test_hook.py
"""

from __future__ import annotations

import io
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from airlock import hook  # noqa: E402
from airlock.types import Verdict  # noqa: E402


# --- extract_pip_packages ---------------------------------------------------------------

def test_extract_basic():
    assert hook.extract_pip_packages("pip install python-pillow") == ["python-pillow"]


def test_extract_variants():
    assert hook.extract_pip_packages("pip3 install requests") == ["requests"]
    assert hook.extract_pip_packages("python -m pip install flask") == ["flask"]
    assert hook.extract_pip_packages("python3 -m pip install flask") == ["flask"]
    assert hook.extract_pip_packages("uv pip install httpx") == ["httpx"]


def test_extract_flags_and_pins():
    assert hook.extract_pip_packages("pip install -U --quiet six") == ["six"]
    assert hook.extract_pip_packages("pip install requests==2.31.0 flask") == [
        "requests==2.31.0", "flask"]


def test_extract_compound_command():
    assert hook.extract_pip_packages("cd /tmp && pip install evil && echo done") == ["evil"]


def test_extract_requirements_file_has_no_names():
    # -r consumes its value; nothing for the hook to check (documented limitation).
    assert hook.extract_pip_packages("pip install -r requirements.txt") == []


def test_extract_target_flag_value_skipped():
    assert hook.extract_pip_packages("pip install --target ./libs six") == ["six"]


def test_extract_non_install_is_empty():
    assert hook.extract_pip_packages("ls -la") == []
    assert hook.extract_pip_packages("git status") == []
    assert hook.extract_pip_packages("pip download requests") == []


# --- resolve_target ---------------------------------------------------------------------

def test_resolve_local_villain():
    target, is_local = hook.resolve_target("python-pillow")
    assert is_local is True
    assert target.endswith("python-pillow")
    assert Path(target).is_dir()


def test_resolve_normalizes_name():
    # PEP 503: underscores/case shouldn't matter for the index lookup.
    target, is_local = hook.resolve_target("Python_Pillow")
    assert is_local is True and Path(target).is_dir()


def test_resolve_registry_name_passthrough():
    target, is_local = hook.resolve_target("requests==2.31.0")
    assert is_local is False
    assert target == "requests"


# --- decide (check() stubbed) -----------------------------------------------------------

def _stub_check(result):
    """Inject a fake `airlock.gate` module so decide()'s `from .gate import check` resolves
    to a stub — no real gate import, so no Daytona/openai needed offline. Returns
    (calls, restore)."""
    import types as _types

    calls = []

    def fake(pkg, **kwargs):
        calls.append(pkg)
        return result

    fake_mod = _types.ModuleType("airlock.gate")
    fake_mod.check = fake
    saved = sys.modules.get("airlock.gate")
    sys.modules["airlock.gate"] = fake_mod

    def restore():
        if saved is not None:
            sys.modules["airlock.gate"] = saved
        else:
            sys.modules.pop("airlock.gate", None)

    return calls, restore


def test_decide_block():
    v = Verdict(package="python-pillow", verdict="BLOCK",
                reasons=["read a planted secret (honeytoken): ~/.env",
                         "connected out to a non-registry host (phoning home): 45.11.87.9:443"])
    calls, restore = _stub_check(v)
    try:
        decision, reason = hook.decide("pip install python-pillow")
    finally:
        restore()
    assert decision == "deny"
    assert "BLOCKED" in reason and "python-pillow" in reason
    assert "honeytoken" in reason
    # the local villain dir was what got detonated
    assert calls and calls[0].endswith("python-pillow")


def test_decide_safe_is_silent():
    v = Verdict(package="six", verdict="SAFE", reasons=["No suspicious behaviour observed."])
    calls, restore = _stub_check(v)
    try:
        decision, reason = hook.decide("pip install six")
    finally:
        restore()
    assert decision is None
    assert calls == ["six"]


def test_decide_error_asks():
    v = Verdict(package="six", verdict="ERROR", error="DAYTONA_API_KEY not set")
    calls, restore = _stub_check(v)
    try:
        decision, reason = hook.decide("pip install six")
    finally:
        restore()
    assert decision == "ask"
    assert "could not complete" in reason


def test_decide_non_install_never_calls_check():
    v = Verdict(package="x", verdict="SAFE")
    calls, restore = _stub_check(v)
    try:
        decision, reason = hook.decide("ls -la")
    finally:
        restore()
    assert decision is None and calls == []


def test_decide_gate_unimportable_asks():
    # Simulate a broken/uninstalled gate: `from .gate import check` fails → never a silent
    # allow for a real install; the human is asked.
    import types as _types
    broken = _types.ModuleType("airlock.gate")  # no `check` attribute
    saved = sys.modules.get("airlock.gate")
    sys.modules["airlock.gate"] = broken
    try:
        decision, reason = hook.decide("pip install python-pillow")
    finally:
        if saved is not None:
            sys.modules["airlock.gate"] = saved
        else:
            sys.modules.pop("airlock.gate", None)
    assert decision == "ask"
    assert "failed to run" in reason


# --- main() full stdin/stdout path ------------------------------------------------------

def _run_main(payload: dict) -> tuple[int, str]:
    old_in, old_out = sys.stdin, sys.stdout
    sys.stdin = io.StringIO(json.dumps(payload))
    sys.stdout = io.StringIO()
    try:
        code = hook.main()
        return code, sys.stdout.getvalue()
    finally:
        sys.stdin, sys.stdout = old_in, old_out


def test_main_deny_emits_json():
    v = Verdict(package="python-pillow", verdict="BLOCK", reasons=["phoned home"])
    _calls, restore = _stub_check(v)
    try:
        code, out = _run_main(
            {"tool_name": "Bash", "tool_input": {"command": "pip install python-pillow"}})
    finally:
        restore()
    assert code == 0
    doc = json.loads(out)
    hso = doc["hookSpecificOutput"]
    assert hso["hookEventName"] == "PreToolUse"
    assert hso["permissionDecision"] == "deny"
    assert "BLOCKED" in hso["permissionDecisionReason"]


def test_main_non_install_is_silent():
    v = Verdict(package="x", verdict="SAFE")
    _calls, restore = _stub_check(v)
    try:
        code, out = _run_main({"tool_name": "Bash", "tool_input": {"command": "ls"}})
    finally:
        restore()
    assert code == 0 and out.strip() == ""


def test_main_non_bash_is_silent():
    code, out = _run_main({"tool_name": "Read", "tool_input": {"file_path": "x"}})
    assert code == 0 and out.strip() == ""


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for fn in fns:
        try:
            fn()
            print(f"  ok  {fn.__name__}")
        except Exception as e:
            failed += 1
            print(f"FAIL  {fn.__name__}: {e!r}")
    print(f"\n{len(fns) - failed}/{len(fns)} passed")
    sys.exit(1 if failed else 0)
