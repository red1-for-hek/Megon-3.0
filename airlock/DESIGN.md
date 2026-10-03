# Airlock — Design Doc

**Status:** Design locked; enforcement is hook interception (§5). Core mechanism proven live on free-tier Daytona — though the proving code (the smoke test) is no longer in the repo (§7).
**Event:** Daytona HackSprint, NUS Singapore, 18 July 2026.
**Required sponsors (reset 2026-07-18):** **Daytona · Nosana · Doubleword · ai& · Oxylabs** — five, each load-bearing, mapped onto the run→read→judge gate (slide's agent-stack metaphor in brackets):
> - **Daytona** *(Hands — runtime & action)* → **RUN** it: detonate the package in the disposable sandbox.
> - **Nosana** *(Muscle — decentralized scale)* → **READ** it: GPU static code-read (+ the fan-out muscle).
> - **Doubleword** *(Memory — recall of past attacks)* → **MATCH** it: embed the source and cosine-match it against a known-malware corpus; a close hit is a distinct evidence stream for the judge (optional — the gate fail-safes without a key).
> - **ai&** *(Brain — sovereign compute)* → **JUDGE** it: weighs the evidence → SAFE/BLOCK. OpenAI-compatible API at `https://api.aiand.com/v1` (`airlock/analysis/llm.py` already speaks it) — ai& is the judge.
> - **Oxylabs** *(Senses — perception & data)* → **REPUTATION**: live web signals on the package (e.g. "is this name reported as malware anywhere?"), layered on top of the free PyPI/OSV signals.
**How to read this doc:** plain text = the build. **SAY** = words to speak on stage, verbatim. *(FYI — …)* = background for us, never presented. *(LATER — …)* = deliberate future work: the next step, and why it's not in the MVP.

**Terms (used throughout):**
- **Orchestrator** — the Airlock program itself, running on your normal machine with full internet. It creates the sandboxes, calls ai&/Nosana/Doubleword/Oxylabs/reputation, and returns the verdict. So "on the orchestrator" = in our controlling program (full internet), and "in the sandbox" = inside the throwaway box.

  What the sandbox's network can and can't do (§8): it can reach package registries (PyPI) and AI APIs, so pip can still download packages — but it *can't* reach arbitrary servers. This doesn't stop the package from running (running code is just local CPU work — no internet needed). What it stops is the malware phoning home: when a bad package tries to send stolen secrets to some random server, the connection fails. And we catch that attempt either way — the audit hook logs the moment it *tries* to connect, before the network even matters (§7).
- **Sandbox** — a throwaway Daytona Linux box, created fresh per check, used for seconds, then deleted. Where we detonate the package so it can't touch the real machine.
- **Hook (a `PreToolUse` hook)** — a small script Claude Code runs *automatically* right before the agent executes any command. ("Hook" = the general programming word for hanging your own code onto a system's normal flow; "PreToolUse" = *before the agent uses a tool*.) What ours is, concretely:
  - **Where it lives:** one entry in Claude Code's `.claude/settings.json` registers it. From then on Claude Code runs it before every command, no agent awareness needed.
  - **How it receives the command:** Claude Code passes the pending command to our script as JSON on standard input — e.g. `{"tool":"Bash","command":"pip install python-pillow"}`.
  - **What our script does:** (1) a **regex** tests the command for `pip install <name>` and pulls out the package name — anything that isn't a pip install (`ls`, `git status`) stops here in ~a millisecond and is allowed; (2) if it *is* an install, it calls the Airlock gate on that package and waits for the SAFE/BLOCK verdict.
  - **How it answers:** the script prints back an allow/deny decision (with the reason on deny); Claude Code obeys — deny means the command never runs.
  - **What it's written in:** a normal program, not bash one-liners — we'd write it in Python so it can call the gate code directly. The regex is just the matching step inside it.

  Because Claude Code runs this before *every* command, no install can slip past — that's how the hook forces every install through the gate.

  *Heads-up — two different things are called "hook" in this doc:* **this** one (the `PreToolUse` hook) is Claude Code intercepting the agent's install *commands*, on the orchestrator. The **audit hook** in §3 (Python's `sys.addaudithook`) is a separate thing — it watches the package's *own actions* from inside the sandbox. Same word, two mechanisms, different layers.
- **MCP (Model Context Protocol)** — a fixed format for an AI agent to call an external program and get a structured answer back. *(In our build MCP is optional — the enforcer is the hook; see "Where MCP actually fits" below and §5.)* Used as the running example here because it's concrete: Airlock *can* run as an MCP *server* advertising one function, `check_package(name)`. The agent is the *client*: it sends a message meaning "run `check_package` with `name=pillow`," Airlock runs its gate logic and replies `{verdict, reasons}`. MCP is just the standardized wiring for that request-and-reply. The payoff: because it's a shared standard, any MCP-compatible agent (Claude Code, others) can use Airlock by adding one config line — no custom integration per agent. (The brief also calls out Daytona's MCP server, so using MCP scores on sponsor fit.) Full walk-through below.

**MCP crash course (FYI — for us, so we can both explain it; skip if it's already clear).** Built from the ground up, because MCP only makes sense once the layer under it is solid:

1. **An agent, alone, only produces text.** It can't actually install a package or read a file — it can only write words. So to *do* anything, it has to ask a separate program to act for it.
2. **"Tool calling" is that hand-off.** When the agent needs to act, it emits a structured request — *"call `check_package` with `pillow`"* — and a separate program runs it and hands back a result. That request-and-reply is a tool call. This is the foundation; if it's fuzzy, MCP won't land.
3. **MCP is just a *standard* for that hand-off.** It doesn't add capability — it's an agreed format for how the request and reply are phrased, so any agent can call any tool without custom glue code written per agent. (Like HTTP or PDF: an agreement between programs, not a program itself. You never see MCP — you see programs speaking it.)
4. **"Server" here means "the side that provides the tools," not a networked box.** Airlock is the MCP *server* even when it's just a script on your laptop; the agent is the *client*. Mentally swap "server" → "tool-provider" and it stops fighting your intuition.

**Where MCP actually fits in our build:** the **hook** is what enforces the gate in the demo — it forces every install through, and the agent can't skip it (§5). MCP is **optional** and does *not* enforce anything on its own (a tool the agent merely *can* call, but might not). We'd only add an MCP tool for on-request checks like the requirements.txt fan-out. So don't read "MCP" as central — the hook is the star; MCP is a maybe.

---

## Event facts (from the official brief — context for anyone touching this doc)

- **What:** Full-day hackathon by AI Builders + NUS StartIT; Daytona is title sponsor. Build a working AI project in one day using the event's sponsor technologies. Airlock's build uses **Daytona, Nosana, Doubleword, ai&, and Oxylabs**, each load-bearing (§6).
- **Agenda:** 10:00 kickoff → 10:30 Daytona workshop + team formation → 11:30 hacking begins → 12:30 lunch → 16:30 hacking ends + **live demos (2 min per team)** → 17:30 winners. **Effective hacking window ≈ 5 hours** — which is why this doc's strategy is "build ahead; always have a working slice" (§9).
- **Teams:** up to 6, or solo; formed after the workshop.
- **Prizes/credits:** winners share sponsor credits; **all participants get sponsor credits to build with on the day** — so free-tier limits (§8) may loosen at the event (pure upside; nothing in the design depends on it).
- **Judging criteria (verbatim):** Completeness (shipped at least an MVP) · Innovation · Real-Life Problem Solving · Sponsored Product Usage. §12 maps how Airlock scores on each.
- **Event theme:** infrastructure for autonomous agents that can "safely execute code, install packages, run servers" — Airlock's positioning (§2) is aimed squarely at this. The brief also highlights that Daytona ships its own MCP server (MCP: see Terms) — the organizers like MCP, so we *may* expose Airlock as an optional MCP tool for a few sponsor points (§5). But note: the demo's enforcement is a **hook**, not MCP.

**The test for every change (to this doc or the build):** does it make the 2-minute live demo on 18 July stronger — a shipped MVP that reads as innovative, solves a real problem, and visibly uses all five sponsors? If not, it doesn't go in.

---

## 0. One line

> Airlock is a safety gate for AI agents. Before an agent installs or runs any code, Airlock detonates it in a disposable sandbox first — so the agent never runs a malicious package on the real machine.

**Guiding principle for the whole build: simplicity wins.** Every feature below is scored on "does this earn its complexity?" If in doubt, cut it. A clean thing that works beats a clever thing that half-works. See §11.

---

## 1. The problem (why this matters now)

AI agents now install packages and run code on their own, constantly, with nobody watching. Meanwhile npm/PyPI are flooded with malicious packages — Sonatype tracked **34,319 new malicious packages in Q3 2025 alone**, and **37% were data-exfiltration** (stealing data off the machine — credentials, tokens, files — and sending it out to the attacker). Most steal credentials the instant they're installed.

**The line that sums it up:**
> "In March 2026, a trojanized package rode into npm right alongside Anthropic's own Claude Code. If it can happen to the people who build Claude, your agent installing packages at 2am doesn't stand a chance without a gate."

**What that means:** when Anthropic shipped Claude Code via npm, a hidden remote-access trojan (malware disguised as safe code that hands an attacker remote control of the machine — "trojan" = looks legit, hides a payload) slipped in through a pulled dependency (one of the helper packages Claude Code automatically installs because it relies on them — so the malware wasn't in Claude Code's own code, it rode in on something Claude Code dragged in). Anyone who installed it that morning ran malware automatically — just by installing. Airlock's job is exactly that moment: run the package in a throwaway sandbox first, catch it phoning home (connecting back out to the attacker's server to deliver stolen data or fetch instructions) / stealing secrets, and block it before it reaches the agent's real machine.

**Honest scope note:** Airlock addresses malicious code that *runs* (install-time trojans — malware that fires the instant you install, since installing runs the package's setup code; credential stealers — malware built to grab passwords / API keys / cloud keys; typosquats — bad packages named to look like a real one so a typo installs them, e.g. `python-pillow` for `pillow`). It does NOT address the *accidental source-code leak* half of that incident — that's a different problem, and we don't claim it.

**Registry note (npm headline, pip demo).** The Claude Code incident happened on npm (JavaScript packages); our MVP guards pip (Python packages). **SAY**, before anyone spots the mismatch: *"That incident was npm; our MVP guards pip — same kind of attack, and our gate checks both the same way."*
(FYI — pip and npm are the same kind of tool for different languages: pip downloads Python packages from the PyPI registry, npm downloads JavaScript packages from the npm registry. A `requirements.txt` is a project's shopping list of pip packages — it's the many-package install §10 checks all at once ("fan-out", §9 Extras). Attackers plant malicious packages in both registries the same way — one attack class.)

(FYI — a pip-native example to reach for if a judge wants a Python case instead of the npm headline: **`fabrice`**. It's a typosquat of `fabric` — a popular, legitimate Python library for running commands on remote servers over SSH — so a fat-fingered `pip install fabrice` gets you the fake. The malicious `fabrice` sat on PyPI for years undetected, pulled tens of thousands of downloads, and on install stole the victim's AWS credentials (cloud access keys) and sent them to the attacker. It's our exact demo scenario in the real world — typosquat name + fires on install + steals cloud credentials — which is proof the demo isn't a strawman we invented.)

---

## 2. The angle (positioning)

**Primary and only target: AI agents.** Not humans pasting, not a web app, not a CI gate (a CI gate is too late — the dev/agent is already compromised on first install). Airlock sits at the agent's **execute boundary**: the instant before it installs or runs code.

Why agents:
- It is the exact theme of this event ("infrastructure built for autonomous agents... so agents can safely execute code").
- It makes Daytona the hero: Daytona gives agents a sandbox to run code; Airlock decides *whether that code is safe to run*.
- It's the freshest, least-crowded framing (static package scanners already exist; an agent safety-gate barely does).

---

## 3. How it works (run it → read it → judge it)

When an agent tries to install a package, Airlock catches it and gathers evidence two independent ways — one watches the code *run* (dynamic), one *reads* the code (static) — then a judge weighs both and returns SAFE or BLOCK. Two checks because each catches what the other misses; that's how real malware analysis works. Here is the whole flow, concretely.

**Step 1 — RUN it (Daytona, dynamic). → produces a behavior log.**
- Airlock creates a fresh disposable sandbox and **plants bait**: fake secret files that look valuable but are worthless — a decoy `~/.aws/credentials`, a decoy `.env` full of fake API keys. These are the *honeytokens*. Nothing legitimate has any reason to touch them.
- It runs the package **under an audit hook** — a built-in Python feature (`sys.addaudithook`; a *different* "hook" from the enforcement one in §5 — this one watches the package from inside the sandbox) that fires on every sensitive action the code takes: opening a file, opening a network connection, spawning a subprocess. Each such action gets written as a line. **This is where the log comes from** — we're not guessing, we're recording what the code actually did.
- The output is a **behavior trace** (the log). From the already-proven run (§7), the real lines look like:
  ```
  Read a SECRET file: /home/daytona/.aws/credentials
  Read a SECRET file: /home/daytona/.env
  Tried to send data OUT to: ('45.11.87.9', 80)
  ```
- *Catches:* attacks that only show themselves when the code runs — obfuscated code, code that fetches its real payload at runtime. You see what it *does*, not what it claims.
- *Blind to:* dormant / time-bomb code that stays quiet during the short run (e.g. "only fire after Aug 1").

**Step 2 — READ it (Nosana, static). → produces a report.**
- A code model on Nosana's GPU reads *all* the package's source — including the branches that never ran in Step 1 — hunting for malicious patterns (reverse shells — code that opens a remote-control channel back to an attacker; exfiltration code; command-and-control logic). It returns a short written report with a suspicion score.
- *Catches:* exactly Step 1's blind spot — the dormant payload sitting in the source that never executed.
- *Blind to:* heavily obfuscated code (deliberately scrambled so a reader can't tell what it does) — which Step 1 catches by running it.

**Step 3 — JUDGE it (ai&). → produces the verdict.**
- ai& is handed four pieces of evidence: the behavior log (Step 1), Nosana's report (Step 2), the Doubleword known-malware similarity match (§6), and a reputation check (package age, downloads, typosquat distance, known CVEs — publicly listed security vulnerabilities — plus Oxylabs web intel — §6). It never sees the package source itself, only these reports.
- It weighs them and returns a plain-English `SAFE` or `BLOCK` with reasons. It's the decision-maker, not a third scanner.

### How the verdict is actually decided (two tiers)

Not every case needs ai&. The verdict is decided in two tiers:

**Tier 1 — deterministic tripwires (plain code, no model).** Some lines in the behavior log are *never* OK during a simple install. Airlock checks the log against a fixed list:
- read a honeytoken (touched a planted fake secret), **or**
- connected out to a non-registry host (phoning home), **or**
- spawned a shell.

If any appears → **instant BLOCK**, decided by plain if-statement code, no model involved. This is exactly the mechanism proven in §7, and it's what the demo's BLOCK rides on. It cannot wobble.

**Tier 2 — ai& judges the gray zone.** If no tripwire fired, the evidence is *ambiguous* and needs judgment — because the same log line means opposite things for different packages:
- reading `.env` is an attack for a typosquat — but it's `python-dotenv`'s entire legitimate job.
- connecting to GitHub is exfiltration for one package — but normal for a package that downloads a data file.
- an odd-looking code path might be a dormant payload — or a harmless self-updater.

A plain rule over the raw log would either miss real attacks or block half of PyPI. So the ambiguous cases go to ai&, which weighs the full picture (log + static report + reputation) and decides.

**Why this stays safe even though ai& isn't deterministic.** The final verdict is `tripwire-BLOCK OR ai&-BLOCK`. ai& can only *add* blocks — it can never overrule a tripwire and flip a BLOCK to SAFE — so the model layer can only make the gate **stricter** than the plain-code rules, never looser. On top of that, ai& is told "if unsure, BLOCK" (a false block just costs the agent a retry; a false pass costs the machine), runs at temperature 0 (as repeatable as a model gets) with a fixed JSON output, and must cite the evidence lines behind every verdict.

Only `SAFE` code reaches the agent. Malicious code ran in the throwaway sandbox and died there when it was deleted.

---

## 4. Architecture

Two places things run. **Everything with full internet runs on the orchestrator; only the dangerous execution runs in the sandbox.** (This layout is dictated by tested free-tier limits — see §8.)

```
                 ┌─────────────────────── ORCHESTRATOR (your machine, full access) ───────────────────────┐
   AI agent ──►  │  Airlock gate:                                                                          │
  (via hook)     │    1. create Daytona sandbox ─────────────►  ┌──── DAYTONA SANDBOX (disposable) ────┐  │
                 │    2. plant honeytokens, run target ───────► │  run code under instrumentation      │  │
                 │    3. pull behavior trace  ◄──────────────── │  (audit hook: file/net/proc events)  │  │
                 │                                              └──────────────────────────────────────┘  │
                 │    4. static scan (call Nosana GPU model)  ◄── reads the source                         │
                 │    5. similarity match (call Doubleword, embed source)                                  │
                 │    6. reputation check (Oxylabs + free PyPI/OSV APIs)                                   │
                 │    7. judge (call ai&) ──► verdict JSON                                                 │
                 │    8. delete sandbox, return SAFE/BLOCK to agent                                        │
                 └────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Agent → Airlock → Daytona.** The agent talks only to Airlock; Airlock owns the sandboxes.
- ai&, Nosana, Doubleword, Oxylabs, and reputation are all called **from the orchestrator** (they're blocked from inside a free sandbox — see §8). Only the detonation is in the sandbox.
- **No persistent Daytona "instance":** one fresh disposable sandbox is created per package check (~0.7s, §7), lives seconds, then deleted — clean attribution per package, and malware dies with the box.

**Keys — who supplies the API credentials.**

*Now (MVP): bring-your-own.* Airlock runs on the user's own machine; they put the five sponsors' keys in `.env` (say this plainly in user-facing docs):
- **Daytona** — the sandboxes.
- **Nosana** — the static-read endpoint.
- **Doubleword** — the embeddings endpoint for the known-malware match (optional; the gate fail-safes to its other legs without it).
- **ai&** — the judge.
- **Oxylabs** — the reputation / web-intel signals (free PyPI/OSV APIs cover it if this key is absent).

*(LATER — hosted service.)* Airlock runs on our servers with our keys. The user just:
- points `PIP_INDEX_URL` at us (the §5 proxy) and holds one Airlock key — no sponsor accounts needed.
- Bonus — one shared cache across all users: each `package@version` is detonated **once, ever**; everyone else's check of it is an instant cache hit. So cost per check trends to zero, and we see brand-new malware across the whole user base before anyone else.

---

## 5. Agent integration (how an agent connects)

**Chosen approach: a hook enforces the gate. MCP is an optional add-on, not the enforcer.** (Earlier drafts leaned on MCP — this is the clear version.)

**The strictness ladder (how the three options relate).** Same gate underneath all three — they differ only in how hard they are to bypass. Climb it for a stronger guarantee:
1. **MCP tool** — opt-in: the agent/user *chooses* to check. Easiest to adopt, weakest guarantee (skippable). Optional (below).
2. **Hook** — automatic: intercepts every install; the agent can't skip it. **This is what we build for the demo.**
3. **Registry proxy** — become the registry: even nested/hidden installs can't dodge it. Production endgame (LATER, below).

We demo **rung 2** — the strongest guarantee you can *show* live ("it installed normally and still got stopped") — and reference rung 1 (adopt-anywhere) and rung 3 (production) as the two ends of the arc.

### What we build: hook interception (this is the enforcer)

A `PreToolUse` hook (a Claude Code feature — see Terms) fires on every shell command the agent runs. It regex-matches `pip install`, extracts the package, detonates it through the gate, then allows the command (SAFE) or blocks it with a reason (BLOCK). The agent needs **zero awareness** of Airlock — every install is screened automatically, like a metal detector. Nothing depends on the model choosing to cooperate.

Performance is a non-issue: the hook only fires on shell calls, and a non-install command costs a ~millisecond regex sniff. Only a real `pip install` pays the detonation cost — inherent to the product any way you build it — and a planned per-`package@version` verdict cache (§9 Extras — not built yet) will make repeat installs instant.

### Why not MCP as the enforcer (the key decision)

MCP would expose the gate as a tool, `check_package(name)`, that the agent *can* call. But a tool sitting in the agent's toolbox does nothing unless the agent **chooses** to call it — and just offering it doesn't guarantee the model uses it. A confused or manipulated agent could skip the check and `pip install` directly, and nothing would stop it. Enforcement can't ride on the model's goodwill. The hook removes the choice: the harness runs it on every command whether the agent likes it or not. **That's the whole reason we picked the hook over MCP.**

### The optional MCP tool (only if we do the fan-out)

MCP has one honest use the hook can't cover: **on-request checks.** The hook only *reacts* to installs — it gives the agent no way to *ask* "evaluate these 20 packages for me." That deliberate request is what the requirements.txt fan-out needs (§9 Extras), and it's what an MCP tool provides. The brief also highlights MCP, so a thin tool scores a few sponsor points.
- **It is optional.** MCP is not a sponsor (Daytona / Nosana / Doubleword / ai& / Oxylabs are), and the core demo — agent installs a bad package, hook blocks it — needs no MCP at all. Build it only if you're doing the fan-out, and only after the core works.
- Config if built: `"mcpServers": { "airlock": { "command": "airlock-server" } }` — a local program on the same machine, no network. ("Server" is just MCP's word for the tool-provider side, local or not.)

### Fallbacks, if the hook fights us at stage time (drop without hesitation)
- **Instructed (the simple floor):** skip the hook; use an MCP tool + two lines in the agent's instructions file (`CLAUDE.md` / `AGENTS.md`): *"always check before installing; never install on BLOCK."* Trivial and works with any agent — but it's the weak version we just argued against (it trusts the model to obey). Fine only as a last-resort demo floor.
- **Deny-list backstop:** harness rules block direct `pip install` so installs are forced through Airlock's tool. An optional extra layer; not in the demo build.

**SAY:** *"instructed today, structurally enforced in production — same engine, stricter plumbing."*

(LATER — registry proxy. *Why the hook is enough for now, and what replaces it in production:*
- **The hook's weak spot:** it matches command *text*. An install buried inside a script — `bash setup.sh` that runs `pip install` internally — has no "pip install" in the text the agent typed, so it slips past. Fine for the MVP: a normal (non-adversarial) agent installs directly, and the hook is ~20 lines.
- **The production fix:** stop matching commands, control *where packages come from*. pip has a setting, `PIP_INDEX_URL`, for which server it downloads from. Point it at an Airlock server, and every install — however deeply nested in scripts — must fetch its files through us, because you can't install what you can't download.
- **What changes:** the proxy becomes the enforcer; the hook shrinks to a thin helper that just (a) delivers the block *reason* into the agent's chat, and (b) catches the rare installs that skip the registry entirely, like `pip install git+https://…`.
- **Carries to other ecosystems:** npm has the same kind of setting, so the same trick works for every branch-out (§11).
- One-liner: *"today we match the command; in production we become the registry."*)

(LATER — distribution, back-of-mind (not MVP). How Airlock reaches other people once it's more than a demo:
- **Ship as an MCP server** so any agent plugs in with one config line — that's rung 1, *reach* — paired with the hook or proxy for *teeth* (rungs 2–3). This is the "instructed today, enforced in production" arc as a product.
- **Auto-setup for the guarded route:** an `airlock init` command that *merges* the hook entry into the user's `.claude/settings.json` (merge, don't overwrite their existing settings) and drops the hook script — or package it as a Claude Code **plugin** that registers the hook on install, so there's no manual editing.
- For the hackathon we skip all of this: hand-wire the two files (hook script + settings entry) in our own demo repo. Installer is polish for when other people start using it.)

---

## 6. Tech / sponsor mapping (each does a different, necessary job)

| Tech | Role | Where it runs | Load-bearing? |
|---|---|---|---|
| **Daytona** (required) | Disposable sandbox — detonate + watch behavior. Also parallel fan-out (check many at once). | Sandbox | Yes — isolation is the product; no Airlock without it. |
| **Nosana** (required) | GPU running an open code model that statically reads the source for malicious patterns. Covers the dynamic blind spot. | Orchestrator | Yes — it's the "read it" half. |
| **Doubleword** (optional) | Embeds the package source and cosine-matches it to a corpus of known-malware patterns — the "have we seen this trick before?" signal. | Orchestrator | Additive — a distinct 4th evidence stream that catches reworded/repackaged variants; the gate fail-safes without it. |
| **ai&** (required) | The judge: weighs behavior + static + match + reputation, writes the verdict. Sovereign-compute inference, OpenAI-compatible API (`api.aiand.com`). | Orchestrator | Yes — the decision-maker + plain-English output. |
| **Oxylabs** (required) | Reputation / web intel — live web signals on the package (name reported as malware? repo health?), layered on top of the free PyPI/OSV APIs (which remain the fallback). | Orchestrator | Yes — the "senses"; catches brand-new malware the fixed free DBs miss. |

**Nosana model:** deploy an open code model (e.g. Qwen2.5-Coder-7B) and prompt it as a security analyst: *"Here is a package's source. Does it contain malicious behaviour? Score + reasons."* Deploy it EARLY (workshop/lunch window), not last — it's the fiddliest piece.

**Why scanner and judge are separate models:** reading thousands of lines of source is high-volume, specialized work — right-sized for a small code-tuned model on a rented GPU; the final call is low-volume frontier reasoning over four short reports (ai& — a strong reasoning model, called as a hosted API rather than something we'd squeeze onto the single Nosana GPU). Separation also keeps the static reader blind to the dynamic trace, so a clean run can't anchor its reading ("run looked fine, that weird `exec()` is probably a self-updater"), and keeps each prompt simple: "find malicious patterns in this code" and "given these four reports, decide."

**Dropped on purpose (simplicity + honesty):** CI gate (too late in the flow — the dev/agent is already compromised on first install).

---

## 7. What's already proven

The core engine is not theoretical — it has been run live on the free-tier account:

- **Detection mechanism works.** In a real Daytona sandbox we planted decoy `~/.aws/credentials` and `.env`, ran simulated malware, and caught:
  - `Read a SECRET file: /home/daytona/.aws/credentials`
  - `Read a SECRET file: /home/daytona/.env`
  - `Tried to send data OUT to: ('45.11.87.9', 80)`
  Mechanism: Python `sys.addaudithook` capturing `open` / `socket.connect` / `subprocess` events. Zero extra installs — pure stdlib, so it runs on the free tier.
- **Sandbox create is fast:** ~0.7s including network round-trip.
- **Note:** the *original* proving scripts (smoke test + card renderer) were deleted and never committed. But the gate has since been **rebuilt from scratch in `airlock/`** and is **live-verified** end to end — steps 1-3 + reputation, both SAFE and BLOCK (see §14 and the README).

---

## 8. Free-tier constraints (tested — these shaped the architecture)

Tested live 2026-07-16 on a self-signup free account:
- **Arbitrary web egress is BLOCKED** from inside a sandbox ("Connection reset by peer"). The per-sandbox `domain_allow_list` is accepted but NOT honored on free tier — no workaround.
- **Reachable from inside a sandbox:** package registries (npm/PyPI/GitHub) and major AI inference APIs (OpenAI, Anthropic — tested reachable).
- **Blocked from inside a sandbox:** Nosana, Oxylabs, Telegram, arbitrary hosts.
- **Compute quota (Tier 1, from Daytona docs — not our test):** $200 free credit (each check is seconds of a ~$0.067/hr sandbox, so effectively unlimited for us) + a shared pool of **10 vCPU / 20 GiB RAM / 30 GiB storage** across all *running* sandboxes → ≈**10 small sandboxes concurrent** (the fan-out cap, §9). Single checks — what we run today — are trivially within this. Rate limit: 300 sandbox creates/min.

**Consequence (already baked into §4):** detonation runs in the sandbox; ai&/Nosana/Oxylabs/reputation run on the orchestrator. Nothing in the design depends on the sandbox reaching the open web — so we don't need the organizers to lift any tier. (If they do provide a higher tier, it's pure upside — you could then detonate live-fetched code too.)

---

## 9. Build plan / todo (check off as we go)

Build in order — keep a working slice at every step (no time estimates; we build ahead). **Top list = everything the hackathon demo needs. Below the line = extras to pursue if there's time; the demo works without them.**

### Must-have — the demo path (build in this order)

- [x] **1. Core gate (MVP).** ✅ _live-verified 2026-07-17: a real Daytona sandbox catches honeytoken reads + phone-home; benign package → SAFE._ `check(package)` → create Daytona sandbox → plant honeytokens → run under the audit hook → collect trace → rule-based verdict (read a secret? connected out? spawned a shell? → BLOCK) → delete sandbox → return. *Mechanism is proven (§7); the old smoke-test code is gone, so build this fresh. This step alone is a working demo.*
- [x] **2. ai& judge + card (+ reputation).** ✅ _judge weighs the evidence streams — trace + static + reputation (PyPI age/downloads, OSV advisories, typosquat detection, **+ Oxylabs web intel**) — into SAFE/BLOCK; terminal card render. Verified end-to-end on the prior judge endpoint; the judge is now **ai&** (OpenAI-compatible, config-only swap in `llm.py`); **re-verified live 2026-07-18** — ai& judged the villain → BLOCK end-to-end._ Trace → ai& → plain-English verdict → render as a card.
- [x] **3. Static read (Nosana / ai& fallback).** ✅ _live-verified 2026-07-17 on a real Nosana endpoint: dashboard template "Qwen 3.5 Models" (Ollama, `qwen3.5:9b`) on an RTX 3060 (~$0.05/h) — villain static read 10/10 → BLOCK. The read fallback (when Nosana is down) is now **ai&** — config-only swap from the prior endpoint. Redeploy Nosana fresh at the event; recipe in §14._ *If the GPU misbehaves live, the demo still stands on Daytona + ai& (§11).*
- [x] **4. Hook + demo agent.** ✅ _built + verified 2026-07-17: `airlock/hook.py` (`PreToolUse`) is registered in `.claude/settings.json` (matches `Bash`). It regex-extracts `pip install <pkg>` (also `pip3` / `python -m pip` / `uv pip`, flags, pins, compound `&&` commands), resolves the name against a local index, calls `check()`, and returns **deny** on BLOCK / **ask** on ERROR (fail-safe to the human — never a silent allow of an unevaluated install) / silent-allow on SAFE. Non-install commands cost a ~ms stdlib-only sniff (the gate isn't even imported — `airlock/__init__` is lazy). 18 unit tests cover parse/resolve/decide/main; the villain trips the real tripwires end-to-end (offline via the actual runner)._ Demo agent is **Claude Code**. This is the enforcer — no MCP needed for the core demo (§5). *Fallbacks: scripted ai&-driven agent if Claude Code fights us; instructed fallback if the hook fights us (§5).*
- [x] **5. Villain package (local).** ✅ _built 2026-07-17: `examples/local-index/python-pillow/` — a benign-looking typosquat whose `setup.py` reads the honeytokens (`~/.aws/credentials`, `~/.env`, `~/.ssh/id_rsa`) and phones home to a dead IP, dressed as "anonymous install telemetry". Detonated offline via the real runner → `read_secret` ×2 + `connect` → BLOCK. The hook resolves the name `python-pillow` to this local package, so `pip install python-pillow` never touches PyPI._ Can't use real PyPI (§10 demo prep).

Demo assets & insurance (all detailed in §10):
- [x] Check real PyPI that `python-pillow` isn't already squatted — ✅ verified 2026-07-17: PyPI returns 404 (unregistered), so a live install can't fetch a stranger's code. Safe to use.
- [ ] Stage-dress the "without Airlock" scene (decoys + a listener that prints "credentials received"), run in a disposable VM.
- [ ] Pre-record one clean run + keep a replay path so nothing depends on live network.
- [x] Raise the hook timeout — ✅ done: 300s in `.claude/settings.json`. *(The "warm verdict cache" half depends on a per-`package@version` cache that isn't built yet — see Extras — so rely on the pre-recorded replay path for rehearsed runs.)*

**Language scope:** Python / pip only — the audit-hook instrumentation is pure-Python and covers the headline typosquat case. npm/shell needs `strace` (an extra, below).

### Extras / stretch — below the line (do if time; the demo works without them)

- [x] **Doubleword similarity match (the MATCH leg).** ✅ _built 2026-07-18: `airlock/analysis/doubleword.py` embeds the package source via Doubleword's OpenAI-compatible embeddings API and cosine-matches it against an inert `CORPUS` of known-malware families — a distinct 4th evidence stream feeding the ai& judge + verdict card. Fully fail-safe: no `DOUBLEWORD_API_KEY` (or any error) → `similarity_match()` returns `None` and the gate runs exactly as before. Wired through config, types (`MalwareMatch`), gate, judge, card + the `--sponsors` self-check; not yet exercised live (no hackathon embedding key)._ The "have we seen this trick before?" leg — see §6 and SPONSORS.md.
- [x] **Parallel fan-out (the visual flex).** ✅ _built 2026-07-18: `airlock/fanout.py` (`check_all` runs `check()` over many packages on a bounded thread pool; `parse_requirements`) + CLI (`python -m airlock -r requirements.txt`, or ≥2 positional packages) + a grid card (`card.render_grid`) — safe packages stay a compact one-line row; each **blocked** one re-prints the full single-package card (why + static + reputation) below the grid. Offline-verified: parse, concurrency cap, input-order results, one bad package isolated, local-index resolution (a `python-pillow` line detonates our villain), grid render, CLI routing (`tests/test_fanout.py`, 8 tests). **Live-verified 2026-07-18:** `examples/requirements-demo.txt` (six, certifi, idna + the villain) → 4 concurrent Daytona sandboxes → 3 SAFE + 1 BLOCK. **Still open:** the MCP tool as the agent-facing driver._ Check a whole `requirements.txt` at once → grid of sandboxes firing simultaneously (§10 step 5). Fan-out = one job spread into many running at once; checks are independent, so 20 packages ≈ one check's wall-clock, not 20×. The dev-facing driver is the CLI above; the *agent*-facing driver is still the **MCP tool** (a `pip install -r requirements.txt` command has no package names for the hook regex to extract).
  - [x] Concurrency cap ≈ **10 small (1-vCPU) sandboxes at once** — Tier 1 shares a 10 vCPU / 20 GiB pool (§8); beyond that they queue. ✅ _enforced: `fanout.resolve_concurrency` defaults to **8** (a notch under the ~10 pool for headroom), overridable via `--concurrency` / `AIRLOCK_FANOUT_CONCURRENCY`; an explicit over-cap value is honoured (event credits may lift the tier) but the CLI warns. Each `check()` still owns one sandbox, so the pool bound = the thread-pool bound._ Confirm the exact number on the day (depends on default sandbox size).
- [ ] **npm / multi-language.** Branch out beyond Python (§11 LATER): add install-command regexes, swap the in-sandbox spy to `strace`, tell Nosana it's reading JavaScript.
- [ ] **Registry proxy.** Production enforcement — become the package registry so nested installs can't slip the hook (§5 LATER).
- [ ] **Deny-list backstop.** Harness deny rules as a second enforcement layer under the hook (§5).
- [ ] **Verdict cache (per `package@version`).** Not built yet — today every check re-detonates. A small cache makes repeat installs instant and lets rehearsed demo runs skip the live path. Referenced as a perf win in §5 / §13 (marked *planned* there until this ships).

---

## 10. Demo (≈2 minutes, live)

1. **Claude Code** gets a task: *"set up image resizing with the python-pillow package."* (The prompt names the typosquat so the install fires on cue — and this is the very agent the March 2026 trojan rode in with, §1.)
2. It runs `pip install python-pillow` — a typosquat of `pillow`.
3. **Without Airlock (show the danger first):** it installs, the setup script runs, the planted secrets get stolen. *"The agent just got the machine owned, and nobody was watching."*
4. **With Airlock:** the install routes through the gate → detonated in Daytona → caught reading the decoy `.env` and trying to phone home → **BLOCK** → card explains why → agent avoids it.
5. **Flex:** agent installs a whole `requirements.txt` → a grid of Daytona sandboxes fires at once. *"Each is a full disposable computer. Daytona starts them in a fraction of a second."*
6. **Close:** *"Daytona gives your agent a sandbox to run code. Airlock decides whether that code is safe to run. It's the seatbelt for autonomous agents — and nobody has it yet."*

**Demo prep (build + verify before stage):**
- **The villain package is ours, served locally.** `python-pillow` can't come from real PyPI (unregistered → pip errors out; and we can't publish real stealer code publicly). Build a local wheel / tiny local index with a benign-looking `setup.py` that reads the decoy `.env` and connects out. Bonus: local means no live-network dependency, matching the insurance line.
- **Check real PyPI for `python-pillow` before the event** — if someone already squats that name, a live `pip install` would fetch a stranger's code on stage. If taken, pick a confirmed-unregistered typosquat name and update §10 step 1–2.
- **The "without Airlock" scene needs stage-dressing (step 3):** plant the decoy `~/.aws/credentials` + `.env` on the demo machine and run a local listener that prints "credentials received from …" so the theft is *visible* — otherwise the scariest moment is invisible. Run that scene in a disposable VM/container, not your real laptop (the malware is ours, but still).

Insurance: pre-record one clean run; keep a cached/replay path so the demo never depends on live network. A real Claude Code agent may resolve to the correct `pillow` unprompted — that's why the step-1 task prompt names `python-pillow` explicitly; rehearse it. Claude Code runs on our own Anthropic account, not event credits. Claude Code hooks have a ~60s default timeout — a cold check (sandbox + run + Nosana + ai&) can brush it and the fan-out will exceed it; the per-hook timeout is already raised to 300s (`.claude/settings.json`), and rehearsed runs should lean on the pre-recorded replay path (a verdict cache is a stretch item, §9 Extras).

---

## 11. Simplicity principles (what we deliberately do NOT build)

- **No CI integration, no SaaS, no dashboard.** One gate, one hook, one demo agent. (MCP tool only if we do the fan-out — §5.)
- **Enforcement (§5): hook interception** — the instructed fallback stays as the guaranteed-simple floor; drop back to it without hesitation if the hook wiring fights us. Never let hook plumbing eat gate/judge/demo time.
- **No multi-language support** — Python/pip only. Why start here: (a) it's the highest-traffic dangerous intersection — AI agents write mostly Python, and PyPI is one of the two flooded registries (§1); (b) Python's stdlib `sys.addaudithook` gives us clean behavior instrumentation with zero installs, so it runs on free tier (§7); (c) the architecture is language-agnostic — detonate/read/judge don't care, so expanding later is normal engineering, not a redesign.
  (LATER — npm support, i.e. what "branch out" means: today Airlock only checks **Python** packages. An agent on a JavaScript project runs `npm install express` — npm is JavaScript's pip — and our regex ignores it, so JS packages currently walk past the gate. Supporting npm = replace the three Python-specific parts, keep everything else: **1)** add `npm install` / `yarn add` / `npx` to the command regexes; **2)** swap the in-sandbox spy — Python's audit hook can only watch Python code, so JS needs `strace`, a Linux tool that watches any program at the OS level (this is the real work); **3)** tell Nosana's model it's reading JavaScript instead of Python. Everything else doesn't care what language the package is: sandbox, honeytokens, ai&, verdict policy, reputation, card.) Don't chase npm/shell/strace unless everything else is done.
- **Rule-based verdict is acceptable if ai& wobbles live** — the demo's BLOCK rides the deterministic tripwire floor (§3), which needs no model. ai&'s gray-zone judging is the product story, not a demo dependency.
- **Deploy Nosana early or drop it gracefully** — the demo must stand on Daytona + ai& if the GPU misbehaves.
- **No reference code to reuse** — the smoke test and card renderer that proved the approach were **deleted** (Appendix), so build fresh from the §7 description; don't go looking for them.

The whole thing is: *create sandbox → run code with fake secrets → watch → judge → delete.* Keep it that simple.

---

## 12. Judging criteria — how we score

- **Completeness (shipped a working MVP):** core engine already proven live; the gate runs end-to-end.
- **Innovation (original):** everyone else *reads* code and guesses; we *run it AND read it* — real dynamic+static analysis, aimed at agents.
- **Real-Life Problem Solving (real pain):** this year's headlines (Anthropic/npm, 34k malicious packages/quarter), and it's about AI agents running unchecked code — the event's exact theme.
- **Sponsored Product Usage:** Daytona (run), Nosana (read), Doubleword (match), ai& (judge), Oxylabs (reputation / web intel) — five sponsors, five different necessary jobs, nothing slapped on.

---

## 13. Honest limitations (know these before a judge asks)

- **No single evasion trick beats us — stacking several might.** Each trick alone is covered by the *other* check, which is the whole point of running both:
  - *dormant* code (stays quiet during the run) → Nosana catches it, because it reads the source that never executed.
  - *obfuscated* code (scrambled so a reader can't parse it) → the live run catches it, because you watch what it actually does.
  - *second-stage* code (downloads its real payload later) → both catch the fetch: the run sees the outbound connection (a tripwire), the read sees the download-and-run pattern.

  The genuinely hard case is *one* package doing all of these at once — dormant **and** obfuscated **and** staged — so nothing fires during the run, the reader can't parse it, and the real payload isn't even in the package to read. That can slip us. But it slips every scanner, and forcing malware to beat both dynamic and static at the same time is exactly why we catch more than a one-method tool.
- **Compromised trusted package** (a real, popular package whose maintainer account got hijacked) defeats the *reputation* signal — it looks legit, old, and widely downloaded. But behavior + static still catch the poisoned code when it runs or gets read.
- **Sandbox-aware malware** (code that checks whether it's being watched and stays innocent if so) can hide from the live run — the static read is the backstop, since reading the source doesn't depend on the code choosing to misbehave.
- **Python/pip only** in the demo; broader language coverage is future work (why Python first: §11).
- **Indirect installs slip the string-matcher** — an install buried in a script doesn't match the hook's regex. Accepted for the MVP; production fixes it at the registry layer (see the §5 LATER: "we become the registry").
- **"The verdict isn't deterministic"** (expect this one). Two-part answer:
  - *The guarantees:* the tripwires are plain code — they always fire, no model involved — and ai& can't whitelist past them. Verdict = tripwire-BLOCK **or** ai&-BLOCK, so the model can only make the gate stricter, never looser (§3). When unsure, ai& defaults to BLOCK, and every verdict cites its evidence.
  - *The deeper point:* you can never write a program that perfectly decides whether any given code is malicious — that's a proven mathematical limit (undecidability). So *every* complete defense ends in a judgment call. The alternative to a non-deterministic judge isn't a deterministic judge — it's *no* judge, which silently passes every dormant payload the rules can't see.
- **"Why two models — couldn't the Nosana model also judge?"** — different jobs want different tools; see §6. Short version: cheap specialized reader on GPU for volume, strongest reasoner for the final call, and keeping the reader blind to the trace prevents anchoring.
- **"What does a check cost / how long?"** — seconds of sandbox compute + one small-GPU read + one embeddings match + one ai& call, with the legs running concurrently; a planned per-`package@version` verdict cache (§9 Extras — not built yet) would pay for each package once, and fan-out (§9 Extras) makes a whole `requirements.txt` ≈ one check's wall-clock. Cost scales with *new packages seen*, not with installs.

State these plainly if asked — knowing your tool's limits reads as credibility, not weakness.

---

## 14. Build notes (gotchas from wiring it up live — 2026-07-17)

Steps 1-3 + reputation are implemented in `airlock/` and **live-verified** end to end (SAFE and BLOCK) against Daytona (+ the prior judge/reader endpoints). The judge is now **ai&** and reputation adds **Oxylabs** — both config-level swaps into the same plumbing (`llm.py` / `reputation.py`), **live-verified 2026-07-18** (full villain check: Daytona detonate + Nosana read 10/10 + Oxylabs web-intel + ai& judge → BLOCK, exit 1). Lessons so they don't bite again:

- **ai& model name:** list what your key can call with `GET {AIAND_BASE_URL}/models` (or `client.models.list()`). We default to **`deepseek-v4-flash`** (cheap, fast, 1M context — good under fan-out concurrency); `deepseek-v4-pro` for stronger reasoning, or the free `qwen3.6-27b` for zero-cost eval runs. Set `AIAND_MODEL`.
- **Cloud-judge overload (HTTP 429 / rate-limit) → the "freeze" — the concurrency issue to respect.** Verified 2026-07-17 (on the prior judge): when the judge endpoint is overloaded/rate-limited, the OpenAI client's default retry-with-backoff, multiplied by `llm.py`'s variant-cycling across two calls per check (static read + judge), dragged a full check to *minutes* of silent hang — and the **fan-out multiplies it**: N concurrent checks = N concurrent judge calls, so an overloaded endpoint stacks the hang N-fold. **Fixed and must stay fixed:** `llm.py` uses `max_retries=0` + a 45s timeout and only falls back to another request variant on a `BadRequestError` (a real capability mismatch) — rate-limit/overload/timeout errors fail fast, so the gate degrades to the tripwire BLOCK in ~8s instead of freezing. `check(on_progress=…)` prints per-stage lines (the CLI shows them) so a healthy run isn't mistaken for a hang. The tripwire floor keeps the verdict correct while the judge is down — you just don't *see* the judge. **ai& note:** ai& is a hosted multi-GPU inference service, so it should absorb concurrent judge calls far better than the single Nosana GPU — but keep the fail-fast guard regardless; never reintroduce retry-storms.
- **Per-model parameter quirks:** some models reject `temperature=0` or `response_format` (the old kimi-k3 allowed only `temperature=1`). `airlock/analysis/llm.py` probes variants and remembers what each model accepts — don't hardcode `temperature=0`. ai&'s DeepSeek models accept `temperature=0` + JSON mode, so the most-specific variant should win on the first try.
- **ai& cost:** pay-per-token, credits auto-deducted, failed requests not billed (`console.aiand.com`). `deepseek-v4-flash` ≈ $0.15/$0.25 per 1M tokens (in/out); `qwen3.6-27b` is free. Each check is fractions of a cent. Event credits are the backstop.
- **Oxylabs (reputation / web intel):** Web Scraper API — POST to `https://realtime.oxylabs.io/v1/queries` with **basic auth** (`OXYLABS_USERNAME` / `OXYLABS_PASSWORD`), body like `{"source": "google_search", "query": "<pkg> pypi malware"}`. `airlock/analysis/oxylabs.py` wraps it with a short timeout and folds web-intel notes into `Reputation.notes`; if the creds are unset or the call errors it returns nothing and the free PyPI/OSV signals carry reputation alone. **Concurrency:** the fan-out fires one Oxylabs call per package concurrently — keep the per-call timeout short and always degrade on error so a slow/blocked scrape can't stall the grid.
- **Doubleword (the MATCH leg):** OpenAI-compatible **embeddings** API (`airlock/analysis/doubleword.py`), same client pattern as ai&/Nosana. Embeds the package source and cosine-matches it against the inert `CORPUS` of known-malware patterns; the corpus is embedded once and cached per process. Fires when `DOUBLEWORD_API_KEY` is set (`DOUBLEWORD_BASE_URL` default `https://api.doubleword.ai/v1`, model `DOUBLEWORD_EMBED_MODEL`); with no key or any error, `similarity_match()` returns `None` and the gate runs on its other legs. Wired end-to-end and covered by `--sponsors`; not yet exercised live (no hackathon embedding key).
- **Nosana:** there's a card + free-credits path (no Solana wallet needed) at deploy.nosana.com. It bills per GPU-hour *while running* — set a max duration and stop it when idle. Optional: ai& does the static read as a fallback when `NOSANA_ENDPOINT` is blank.
- **Nosana deploy recipe (worked 2026-07-17):** Deploy → template **"Qwen 3.5 Models"** (Ollama, model `qwen3.5:9b`, needs 9 GB VRAM) → Deployment Configuration: strategy **SIMPLE** (not Infinite — that renews forever), **1 h** timeout, 1 replica → GPU: cheapest ≥12 GB (RTX 3060, ~$0.05/h; 3090 as fallback). Ready in ~10 min (6.5 GB model pull). Then `.env`: `NOSANA_ENDPOINT=<endpoint-url>/v1` (**append `/v1`** — Ollama's OpenAI-compatible path), `NOSANA_MODEL=qwen3.5:9b`, no API key (Ollama has no auth; the endpoint is public but unguessable and dies with the deployment). First call ~70 s (loads the model into VRAM) — that **exceeds `llm.py`'s 45 s timeout, so the *first* real check silently falls back to ai&** (health check `/api/tags` returns 200 the whole time, so "up" ≠ "warm"). Verified live 2026-07-17: cold → `[ai& (Nosana fallback)]`; after one warm-up request → `[Nosana]`. **Send a warm-up request right before the demo** so Nosana, not the fallback, does the read (Ollama keeps the model hot ~5 min).
- **Fan-out reads vs a single-GPU Nosana (measured 2026-07-18) — the read-side concurrency issue.** The fan-out (§9 Extras) runs N checks at once, each doing its own static read. On the free RTX 3060, concurrent reads queue on the one GPU and each blows past `llm.py`'s 45s read timeout → they fall back to ai&. Measured on `examples/requirements-demo.txt` (4 pkgs): `--concurrency 4` ≈ 94s, **reads all fall back** (one GPU can't serve 4 reads at once); `--concurrency 1` ≈ 223s, **Nosana on 3/4** (a heavy read like `certifi` still times out); a single check ≈ 53s, **Nosana**. This is the graceful degrade working, not a bug — the tripwire floor means the verdict stays correct either way. **Decision — don't build a read-gate:** the single-villain check is the four-sponsor hero (Daytona runs · **Nosana reads 10/10** · ai& judges → BLOCK · Oxylabs reputation); the parallel fan-out is the Daytona *scale* flex (fast; reads quietly fall back to ai& under GPU contention). The read is the only GPU-bound leg, so the honest scaling line is *"one free GPU today; a bigger GPU / more replicas runs the deep reads fully parallel."*
- **Daytona:** the free credit (~$100) is effectively unlimited for us — a check costs fractions of a cent. The pool caps concurrency at ≈10 small sandboxes (§8), which only matters for the fan-out.
- **Minimum keys / degradation:** Daytona alone = a working tripwire gate that blocks the demo villain; ai& is *additive* (reasoning + static-read fallback + weighing reputation); Oxylabs is *additive* on top of the free reputation APIs. No Daytona → `check()` returns ERROR. No ai& → reputation is still shown but doesn't change the verdict (a dormant typosquat would pass). No Oxylabs → reputation still runs on the free PyPI/OSV APIs. No Doubleword → the known-malware similarity match is skipped; every other leg is unchanged.
- **Local packages** (the demo villain) are tarred and shipped into the sandbox to detonate; registry names install from PyPI directly.
- **Hook fast path must stay import-light.** Claude Code runs the hook before *every* Bash command, so the non-install path can't afford to import Daytona/OpenAI. `airlock/__init__.py` is lazy (PEP 562 `__getattr__`) and `hook.py` defers `from .gate import check` until an install is actually seen — otherwise `python -m airlock.hook` would import the whole gate (and crash on every command if deps are missing).
- **Hook needs the deps on its interpreter.** `.claude/settings.json` runs `python3 -m airlock.hook`; that `python3` must be the one with Airlock's deps installed (run Claude Code with the venv active, or point the hook `command` at `.venv/bin/python`). If the gate can't import/run, the hook returns **ask**, never a silent allow.
- **Local index = how the typosquat demo dodges PyPI.** The hook maps a package *name* to a local package dir under `examples/local-index/` (override with `AIRLOCK_LOCAL_INDEX`) when one exists, else treats it as a registry name. That's why `pip install python-pillow` detonates our local villain instead of erroring on unregistered PyPI.
- **macOS CA bundle (`CERTIFICATE_VERIFY_FAILED`).** The python.org macOS builds ship no CA bundle, so *every* HTTPS call (Daytona, ai&, Oxylabs) dies with `certificate verify failed` until certs are installed. `airlock/config.py::_ensure_ca_bundle()` fixes it automatically — points `SSL_CERT_FILE`/`REQUESTS_CA_BUNDLE` at `certifi` when the platform default is missing (so the CLI *and* the hook subprocess work out of the box). The system-wide alternative is running Python's "Install Certificates.command" once.

---

## Appendix — repo pointers

- **Gone — the smoke test (`baitbox_smoketest.py`) and card renderer (`replay/baitbox_replay.py`) no longer exist** (and were never committed to git, so not recoverable from history). They held the proven Daytona SDK calls (create / run / download / delete + egress test) and the verdict-card renderer. The §7 result still stands — it *was* run live — but the code isn't in the repo, so don't rely on any "reuse/existing" references (§9, §11); build fresh if you need it.
- **Code (rebuilt, live-verified):** `airlock/` — `sandbox/` (detonate / runner / tripwires), `analysis/` (judge / static_read / doubleword / reputation / oxylabs / llm), plus `gate.py`, `card.py`, `cli.py`, and `hook.py` (the `PreToolUse` enforcer, step 4). `.claude/settings.json` registers the hook. `examples/evil-demo/` is a malicious test package; `examples/local-index/python-pillow/` is the demo villain (step 5). `tests/test_hook.py` covers the hook offline. Run instructions + the `check()` contract: `README.md`.
- `.env` — API keys, git-ignored (Daytona + Nosana + Doubleword + ai& + Oxylabs per §4).
- Pitch stats + incident facts live in this doc: §1 (incident, 34k/quarter) and §8 (tested free-tier limits).
