# Forge Landing Page — Design Spec

**Date:** 2026-04-14
**Status:** Approved

---

## Overview

A dark, premium marketing landing page for **Forge** — an autonomous AI software-engineering agent written in Rust. The page lives at `forge/landing/` inside the monorepo as a standalone Next.js app.

Its goals:
1. Explain what Forge does in under 30 seconds of reading
2. Show users how it works visually
3. Walk users through local setup including OpenAI-compatible model configuration

---

## Tech Stack

| Concern | Choice |
|---|---|
| Framework | Next.js 14 (App Router), TypeScript |
| Styling | Tailwind CSS + `tailwindcss-animate` |
| Fonts | `Inter` (body) + `JetBrains Mono` (code) via `next/font` |
| Icons | `lucide-react` |
| Location | `forge/landing/` |

---

## Theme

- **Background:** `#0d0d0d` (dark charcoal)
- **Primary accent:** Electric blue `#3b82f6`
- **Secondary accent:** Purple `#7c3aed`
- **Gradient:** blue → purple, used on headlines, glows, and CTA buttons
- **Text:** White primary, `#a1a1aa` muted
- **Code blocks:** `#111827` background, monospace font, syntax-highlighted

---

## Navbar

- Sticky, full-width
- Transparent on top of hero, transitions to `#0d0d0d/90` with backdrop blur on scroll
- Left: Forge logo (text + small icon)
- Right: `GitHub` link + `Get Started` CTA button (gradient)
- Smooth scroll anchor links to page sections

---

## Page Sections

### 1. Hero

- **Headline:** "Your autonomous AI software engineer"
- **Subheading:** 2 lines — Forge takes a GitHub issue, spins up an isolated Docker sandbox, autonomously writes and tests code, and produces a verified git diff ready to merge.
- **CTAs:** `Get Started` (scrolls to #setup) + `View on GitHub` (external)
- **Background:** Subtle radial gradient glow (blue → purple) behind headline, low opacity
- **Below fold hint:** Animated scroll indicator

### 2. What Forge Does — Feature Cards

3-column grid (collapses to 1 on mobile). Each card: icon, title, 1-sentence description.

| Title | Description |
|---|---|
| Docker Sandbox | Every run is fully isolated — no leftover state, no host pollution |
| Any OpenAI-compatible Model | Works with OpenAI, Gemini, Anthropic, Ollama, or any compatible endpoint |
| Autonomous Agent Loop | Thinks, acts, observes, repeats until the task is done or the step limit is reached |
| Full Trajectory Recording | Every step, command, and model response saved to a `.traj` file |
| ElizaOS Integration | Drop-in action plugin for ElizaOS AI agents |
| Nosana Deployment | Deploy to decentralized GPU compute with a single JSON job definition |

### 3. How It Works — Terminal + Steps

Split layout (terminal left, steps right on desktop; stacked on mobile).

**Left:** Animated fake terminal that types out the 8-step demo sequence from the README (ls, cat, submit commands), scrolling at a readable pace. Loops.

**Right:** Numbered step list:
1. Fetch the problem statement (GitHub issue, text, or file)
2. Start an isolated Docker sandbox
3. Clone the repository
4. Enter the agent loop — think, act, observe
5. Execute bash commands autonomously
6. Run `submit` to capture the git diff
7. Save the full trajectory

### 4. Local Setup — Numbered Steps with Code Blocks

**Prerequisites block:**
- Docker 24+
- Rust 1.82+
- Command to build sandbox image: `docker build -f Dockerfile.sandbox -t forge-sandbox:latest .`

**Step 1 — Clone & build:**
```bash
git clone <repo-url> && cd forge
cargo build --release -p forge
```

**Step 2 — Configure your model:**
```dotenv
FORGE_MODEL=your-model-name
FORGE_BASE_URL=https://your-provider.example.com/v1/openai
FORGE_API_KEY=your-api-key
```

**Step 3 — Run against a GitHub issue:**
```bash
set -a && source .env && set +a
./target/release/forge run --github-url https://github.com/owner/repo/issues/42
```

### 5. Model Configuration — Dedicated Section

Explains that Forge works with **any OpenAI-compatible API endpoint**. Includes:

- Named compatible providers: OpenAI, Google Gemini, Anthropic (via compatible proxy), Ollama (local)
- `.env` variable reference table: `FORGE_MODEL`, `FORGE_BASE_URL`, `FORGE_API_KEY`
- YAML config snippet showing the `agent:` block with `model_name`, `base_url`, `api_key`, `max_steps`, `parser_type`
- Note: CLI flags override YAML values

### 6. Footer

- MIT License badge
- GitHub link
- ElizaOS logo/link
- Nosana logo/link
- Copyright line

---

## Responsive Behavior

| Breakpoint | Layout change |
|---|---|
| `sm` (640px) | Single column cards |
| `md` (768px) | 2-column cards, terminal/steps stack |
| `lg` (1024px) | 3-column cards, terminal beside steps |

---

## Animations

- **Scroll reveal:** Sections fade up on enter (`IntersectionObserver` + Tailwind animate classes)
- **Terminal:** Character-by-character typing loop using `useEffect` + `setInterval`
- **Hero glow:** CSS `@keyframes` pulse on the background gradient, subtle (opacity 0.3–0.6)
- **CTA button:** Gradient shimmer on hover

---

## File Structure

```
forge/landing/
├── app/
│   ├── layout.tsx          # root layout, fonts, metadata
│   └── page.tsx            # single page, imports all sections
├── components/
│   ├── Navbar.tsx
│   ├── Hero.tsx
│   ├── Features.tsx
│   ├── HowItWorks.tsx
│   ├── Setup.tsx
│   ├── ModelConfig.tsx
│   └── Footer.tsx
├── lib/
│   └── terminal-lines.ts   # demo command sequence for animation
├── tailwind.config.ts
├── next.config.ts
└── package.json
```

---

## Out of Scope

- Authentication or user accounts
- Backend API routes
- Dynamic data fetching
- i18n / localization
- Analytics integration
