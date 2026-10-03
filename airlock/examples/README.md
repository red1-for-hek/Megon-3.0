# examples

Test artifacts for exercising the gate.

- **`evil-demo/`** — a deliberately "malicious" local package (safe to run; the payload only
  reads the planted honeytokens and pings a dead IP). Use it to verify the full package-mode
  flow — `pip install` → `setup.py` runs → detonation catches it → **BLOCK**:

  ```bash
  python -m airlock examples/evil-demo      # expect BLOCK
  ```

  This is a minimal stand-in that proves the mechanism end-to-end.

- **`local-index/`** — the hook's "tiny local index" (build plan §9 step 5). Each subdirectory is
  a local package the enforcement hook detonates *by name* instead of hitting PyPI. It holds:

  - **`python-pillow/`** — the demo villain: a benign-looking typosquat of `pillow` whose
    `setup.py` reads the planted honeytokens and phones home (to a dead IP), dressed up as
    "anonymous install telemetry". Because it lives here, `pip install python-pillow` resolves to
    this local package — so the demo never depends on a real PyPI registration, and a live install
    can't fetch a stranger's code. Detonating it returns **BLOCK**:

    ```bash
    python -m airlock examples/local-index/python-pillow   # expect BLOCK
    ```

  Point the hook at a different index dir with `AIRLOCK_LOCAL_INDEX`. Any package name *without* a
  matching directory here is treated as a normal registry name and installed from PyPI.
