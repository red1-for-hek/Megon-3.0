"""Builds the self-contained detonation script that runs *inside* the Daytona sandbox.

Two pieces:
  * SITECUSTOMIZE_SRC — the audit hook (sys.addaudithook). It is dropped on PYTHONPATH
    so it loads into every Python subprocess (pip install, setup.py, import) and logs
    the package's sensitive actions to a trace file.
  * RUNNER_SRC — orchestration: plant honeytokens, wire the hook, detonate, collect the
    trace, print it as JSON.

build_runner() returns the whole thing as one string, ready for sandbox.process.code_run().
"""

from __future__ import annotations

import base64
import json

RESULT_MARKER = "AIRLOCK_RESULT "

# --- runs in every audited subprocess, via PYTHONPATH ---
SITECUSTOMIZE_SRC = r'''
import sys, os, json, ipaddress

_TRACE = os.environ.get("AIRLOCK_TRACE")
_secrets = set()
for _s in os.environ.get("AIRLOCK_SECRETS", "").split(os.pathsep):
    if _s:
        try:
            _secrets.add(os.path.realpath(_s))
        except Exception:
            pass
_allowed = set(x for x in os.environ.get("AIRLOCK_ALLOWED_IPS", "").split(",") if x)
_busy = False


def _emit(kind, detail):
    try:
        with open(_TRACE, "a") as _f:
            _f.write(json.dumps({"kind": kind, "detail": str(detail)}) + "\n")
    except Exception:
        pass


def _suspicious(host):
    h = str(host)
    if h in _allowed:
        return False
    try:
        ip = ipaddress.ip_address(h)
        return not (ip.is_private or ip.is_loopback or ip.is_link_local or ip.is_multicast)
    except ValueError:
        return h not in ("localhost",)


def _hook(event, args):
    global _busy
    if _busy or not _TRACE:
        return
    _busy = True
    try:
        if event == "open" and args:
            p = args[0]
            if isinstance(p, (str, bytes)):
                try:
                    rp = os.path.realpath(os.fsdecode(p))
                except Exception:
                    rp = ""
                if rp in _secrets:
                    _emit("read_secret", os.fsdecode(p))
        elif event == "socket.connect" and len(args) >= 2:
            addr = args[1]
            host = addr[0] if isinstance(addr, tuple) and addr else None
            if host is not None and _suspicious(host):
                if isinstance(addr, tuple) and len(addr) >= 2:
                    _emit("connect", "%s:%s" % (addr[0], addr[1]))
                else:
                    _emit("connect", str(addr))
        elif event == "os.system" and args:
            _emit("spawn_shell", os.fsdecode(args[0]) if isinstance(args[0], (str, bytes)) else str(args[0]))
        elif event == "subprocess.Popen" and args:
            exe = args[0]
            try:
                name = os.path.basename(os.fsdecode(exe)) if isinstance(exe, (str, bytes)) else ""
            except Exception:
                name = ""
            if name in ("sh", "bash", "zsh", "dash", "ksh"):
                _emit("spawn_shell", os.fsdecode(exe) if isinstance(exe, (str, bytes)) else str(exe))
    except Exception:
        pass
    finally:
        _busy = False


sys.addaudithook(_hook)
'''

# --- orchestration; runs once in the sandbox. Reads CONFIG (injected as a header). ---
RUNNER_SRC = r'''
import os, sys, json, subprocess, tempfile, socket

MODE = CONFIG["mode"]
TARGET = CONFIG["target"]
SITECUSTOMIZE = CONFIG["sitecustomize"]
INSTALL_TIMEOUT = CONFIG.get("install_timeout", 120)
IMPORT_TIMEOUT = CONFIG.get("import_timeout", 20)

HOME = os.path.expanduser("~")
AWS_CRED = os.path.join(HOME, ".aws", "credentials")
ENV_FILE = os.path.join(HOME, ".env")
SECRETS = [AWS_CRED, ENV_FILE]


def plant_honeytokens():
    os.makedirs(os.path.dirname(AWS_CRED), exist_ok=True)
    with open(AWS_CRED, "w") as f:
        f.write("[default]\n"
                "aws_access_key_id = AKIAHONEYTOKEN0000000\n"
                "aws_secret_access_key = wJalrHoneytokenFAKEsecretkey0000000000000\n")
    with open(ENV_FILE, "w") as f:
        f.write("OPENAI_API_KEY=sk-honeytoken00000000000000000000\n"
                "DATABASE_URL=postgres://prod:pw@db.internal:5432/main\n"
                "GITHUB_TOKEN=ghp_honeytoken0000000000000000000000\n")


def resolve_allowlist():
    ips = set()
    for domain in ("pypi.org", "files.pythonhosted.org", "pythonhosted.org", "pypi.python.org"):
        for port in (443, 80):
            try:
                for res in socket.getaddrinfo(domain, port):
                    ips.add(res[4][0])
            except Exception:
                pass
    return ips


def guess_module(pkg):
    base = pkg.split("==")[0].split(">=")[0].split("[")[0].strip()
    base = os.path.basename(base)
    for suf in (".tar.gz", ".whl", ".zip"):
        if base.endswith(suf):
            base = base[: -len(suf)].split("-")[0]
    return base.replace("-", "_")


def read_trace(trace_file):
    events = []
    try:
        with open(trace_file) as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        events.append(json.loads(line))
                    except Exception:
                        pass
    except Exception:
        pass
    seen, uniq = set(), []
    for e in events:
        key = (e.get("kind"), e.get("detail"))
        if key not in seen:
            seen.add(key)
            uniq.append(e)
    return uniq


def main():
    workdir = tempfile.mkdtemp(prefix="airlock_")
    trace_file = os.path.join(workdir, "trace.jsonl")
    open(trace_file, "w").close()

    audit_dir = os.path.join(workdir, "audit")
    os.makedirs(audit_dir, exist_ok=True)
    with open(os.path.join(audit_dir, "sitecustomize.py"), "w") as f:
        f.write(SITECUSTOMIZE)

    plant_honeytokens()
    allowed = resolve_allowlist()

    env = dict(os.environ)
    prev_pp = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = audit_dir + (os.pathsep + prev_pp if prev_pp else "")
    env["AIRLOCK_TRACE"] = trace_file
    env["AIRLOCK_SECRETS"] = os.pathsep.join(SECRETS)
    env["AIRLOCK_ALLOWED_IPS"] = ",".join(sorted(allowed))

    steps = []
    site = os.path.join(workdir, "site")

    if MODE == "code":
        snippet = os.path.join(workdir, "target.py")
        with open(snippet, "w") as f:
            f.write(TARGET)
        try:
            r = subprocess.run([sys.executable, snippet], env=env,
                               capture_output=True, text=True, timeout=INSTALL_TIMEOUT)
            steps.append({"step": "run_code", "returncode": r.returncode,
                          "stderr_tail": (r.stderr or "")[-400:]})
        except subprocess.TimeoutExpired:
            steps.append({"step": "run_code", "error": "timeout"})
    else:
        # A local package is shipped in as a base64 tarball (the sandbox has no access to the
        # orchestrator's filesystem); extract it and install from there. Otherwise TARGET is a
        # registry name pip installs directly.
        install_target = TARGET
        payload = CONFIG.get("local_payload")
        if payload:
            import base64 as _b64, io as _io, tarfile as _tf
            pkgdir = os.path.join(workdir, "pkg")
            os.makedirs(pkgdir, exist_ok=True)
            with _tf.open(fileobj=_io.BytesIO(_b64.b64decode(payload))) as _t:
                _t.extractall(pkgdir)
            install_target = pkgdir

        # Ensure the build backend exists so --no-build-isolation can run setup.py under our
        # audit hook (isolated builds would not load our sitecustomize). setuptools/wheel are
        # wheels, so this triggers no false events.
        try:
            subprocess.run([sys.executable, "-m", "pip", "install", "-q",
                            "--disable-pip-version-check", "setuptools", "wheel"],
                           env=env, capture_output=True, text=True, timeout=90)
        except Exception:
            pass
        try:
            r = subprocess.run(
                [sys.executable, "-m", "pip", "install", "--no-input",
                 "--disable-pip-version-check", "--no-build-isolation", "--target", site, install_target],
                env=env, capture_output=True, text=True, timeout=INSTALL_TIMEOUT)
            steps.append({"step": "pip_install", "returncode": r.returncode,
                          "stderr_tail": (r.stderr or "")[-400:]})
        except subprocess.TimeoutExpired:
            steps.append({"step": "pip_install", "error": "timeout"})
        module = guess_module(TARGET)
        env2 = dict(env)
        env2["PYTHONPATH"] = site + os.pathsep + env["PYTHONPATH"]
        try:
            r2 = subprocess.run([sys.executable, "-c", "import " + module],
                                env=env2, capture_output=True, text=True, timeout=IMPORT_TIMEOUT)
            steps.append({"step": "import", "module": module, "returncode": r2.returncode})
        except Exception as e:
            steps.append({"step": "import", "module": module, "error": str(e)})

    print(RESULT_MARKER + json.dumps({"events": read_trace(trace_file), "steps": steps}))


RESULT_MARKER = "''' + RESULT_MARKER + r'''"
main()
'''


def build_runner(mode: str, target: str, *, install_timeout: int = 120,
                 import_timeout: int = 20, local_payload: str | None = None) -> str:
    """Return the full detonation script for sandbox.process.code_run().

    mode: "package" (pip install a registry name, or a local package shipped in via
    local_payload) or "code" (run a raw snippet).
    """
    config = {
        "mode": mode,
        "target": target,
        "sitecustomize": SITECUSTOMIZE_SRC,
        "install_timeout": install_timeout,
        "import_timeout": import_timeout,
        "local_payload": local_payload,
    }
    payload = base64.b64encode(json.dumps(config).encode()).decode()
    header = (
        "import json as _json, base64 as _b64\n"
        f"CONFIG = _json.loads(_b64.b64decode('{payload}').decode())\n"
    )
    return header + RUNNER_SRC
