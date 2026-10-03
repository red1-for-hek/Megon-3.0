# Sponsor Integrations

Airlock's gate is not a wrapper around one API — each sponsored product does a **different,
load-bearing job**, and the verdict is built from all of them. This file maps every
integration to the exact code that calls it, so you can verify it's real rather than
decorative.

**Verify them live** (pings each endpoint, prints which are reachable — never prints secrets):

```bash
python -m airlock --sponsors
```

| Sponsor | Job in Airlock | Wired in | The real call |
|---|---|---|---|
| **Daytona** | **Runs it** — detonates the package in a throwaway sandbox and records what it does | [`airlock/sandbox/detonate.py`](airlock/sandbox/detonate.py) | `Daytona().create()` → `sandbox.process.code_run()` → `Daytona().delete()` |
| **Nosana** | **Reads it** — a code model on a GPU statically reads the whole source | [`airlock/analysis/static_read.py`](airlock/analysis/static_read.py) | OpenAI-compatible chat to `NOSANA_ENDPOINT` with `NOSANA_MODEL` |
| **Doubleword** | **Matches it** — embeds the source and compares it to known-malware fingerprints | [`airlock/analysis/doubleword.py`](airlock/analysis/doubleword.py) | OpenAI-compatible `embeddings` to `DOUBLEWORD_BASE_URL` with `DOUBLEWORD_EMBED_MODEL` |
| **ai&** | **Judges it** — weighs the four evidence streams into SAFE / BLOCK + reasons | [`airlock/analysis/judge.py`](airlock/analysis/judge.py) | OpenAI-compatible chat to `AIAND_BASE_URL` with `AIAND_MODEL` |
| **Oxylabs** | **Investigates it** — live web intel: is the name being reported as malware anywhere? | [`airlock/analysis/oxylabs.py`](airlock/analysis/oxylabs.py) | POST to `realtime.oxylabs.io/v1/queries` (Web Scraper API) |

The single source of truth for this table is [`airlock/sponsors.py`](airlock/sponsors.py) —
the same registry that powers the self-check.

---

## Each integration, in detail

### 🧨 Daytona — *runs it*
- **Where:** `airlock/sandbox/detonate.py` → `detonate()`.
- **What it does:** creates a fresh disposable Linux sandbox (~0.7s), ships the package in,
  installs it under a `sys.addaudithook` instrumentation that records every file open, socket
  connect, and subprocess spawn, then deletes the sandbox. The behaviour trace it returns is
  Airlock's dynamic evidence.
- **Remove it and:** `check()` returns `ERROR` — there is no gate. Isolation *is* the product.
- **Verify:** `--sponsors` calls `Daytona().list()` (authenticated, creates nothing).

### 🔍 Nosana — *reads it*
- **Where:** `airlock/analysis/static_read.py` → `static_read()` → `_analyze()`.
- **What it does:** downloads the package source and asks a code model, hosted on a Nosana GPU,
  to score it 0–10 for malicious patterns — covering the dormant code that never executed during
  the sandbox run (the dynamic blind spot).
- **Remove it and:** the static half falls back to ai& reading the source; if neither is set,
  the gate runs on the tripwire floor + reputation alone, and a *dormant* typosquat could pass.
- **Verify:** `--sponsors` sends a one-token chat to `NOSANA_ENDPOINT`.
- **Note:** if `NOSANA_ENDPOINT` is unset, this path silently uses ai& instead — so **set and warm
  the Nosana endpoint** for the real integration to fire (otherwise it's present but never runs).

### 🧬 Doubleword — *matches it*
- **Where:** `airlock/analysis/doubleword.py` → `similarity_match()`.
- **What it does:** embeds the package source with a code-aware embedding model on Doubleword and
  cosine-matches it against a small corpus of known install-time-malware patterns (`CORPUS` in that
  file). A close match — e.g. *88% to `aws-credential-stealer`* — is a distinct 4th evidence stream
  that catches repackaged variants which reworded their code to dodge a scanner.
- **Why it doesn't overlap ai&/Nosana:** it's an `embeddings` call, not a `chat` call — a different
  *kind* of operation, answering "have we seen this trick before?" rather than reading or judging.
- **Remove it and:** the gate loses the known-malware similarity signal (still runs on run/read/judge).
- **Verify:** `--sponsors` sends a tiny `embeddings` request and reports the vector dimensions.
- **Note:** the integration is complete and self-checked; it fires when `DOUBLEWORD_API_KEY` is set (a
  hackathon embedding credit), and the gate fail-safes to run/read/judge without one — so it's wired
  end-to-end and activates the moment a key lands, exactly like the Nosana endpoint above.

### ⚖️ ai& — *judges it*
- **Where:** `airlock/analysis/judge.py` → `judge()`.
- **What it does:** receives the behaviour trace, the static-read score, and the reputation
  signals (never the raw source), and returns a plain-English `SAFE`/`BLOCK` with cited reasons.
  It decides the gray zone; it **cannot** overrule a tripwire — the monotonic combine in
  `gate.py` means the model can only ever make the gate *stricter*.
- **Remove it and:** the gate still blocks on the deterministic tripwire floor, but loses its
  reasoning over ambiguous cases and its plain-English explanations.
- **Verify:** `--sponsors` sends a one-token chat to `AIAND_BASE_URL`.

### 🌐 Oxylabs — *investigates it*
- **Where:** `airlock/analysis/oxylabs.py` → `web_intel()`, merged into reputation in
  `airlock/analysis/reputation.py`.
- **What it does:** queries the Web Scraper API for live web signals about the package name —
  catching brand-new malware that fixed advisory databases (OSV) haven't catalogued yet.
- **Remove it and:** reputation falls back to the free PyPI / pypistats / OSV APIs alone, losing
  the live-web signal for zero-day typosquats.
- **Verify:** `--sponsors` runs a minimal authenticated Web Scraper query.

---

## How the evidence combines

```
Daytona    (runs it)      ─┐
Nosana     (reads it)      ─┤
Doubleword (matches it)    ─┼─►  ai& (judges it)  ─►  SAFE / BLOCK
Oxylabs    (investigates)  ─┘        ▲
                                     │
   deterministic tripwire floor (plain code, no model) can BLOCK on its own;
   ai& can only add blocks, never remove one.
```

All five sponsors are wired and load-bearing — run `python -m airlock --sponsors` to see which
are live in your environment.
