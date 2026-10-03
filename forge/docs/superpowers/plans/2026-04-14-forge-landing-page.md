# Forge Landing Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a dark, premium Next.js 14 landing page for Forge at `landing/` inside the monorepo.

**Architecture:** Single-page Next.js 14 app (App Router) with one route (`/`). All content is split into focused React components, one per page section. Scroll-reveal animations use a shared `useInView` hook. The terminal animation in HowItWorks is self-contained with a typed character loop.

**Tech Stack:** Next.js 14, TypeScript, Tailwind CSS, tailwindcss-animate, lucide-react, next/font (Inter + JetBrains Mono)

---

## File Map

| File | Purpose |
|---|---|
| `landing/package.json` | Dependencies |
| `landing/next.config.ts` | Next.js config |
| `landing/tailwind.config.ts` | Theme: colors, fonts, keyframes |
| `landing/app/layout.tsx` | Root layout: fonts, metadata, global styles |
| `landing/app/globals.css` | Tailwind directives + custom keyframes |
| `landing/app/page.tsx` | Assembles all section components |
| `landing/lib/terminal-lines.ts` | Static array of terminal demo lines |
| `landing/hooks/useInView.ts` | Reusable IntersectionObserver hook |
| `landing/components/Navbar.tsx` | Sticky navbar with scroll-aware background |
| `landing/components/Hero.tsx` | Headline, subheading, CTAs, glow background |
| `landing/components/Features.tsx` | 3-col feature cards grid |
| `landing/components/HowItWorks.tsx` | Animated terminal + numbered steps |
| `landing/components/Setup.tsx` | Prerequisites + 3-step local setup |
| `landing/components/ModelConfig.tsx` | OpenAI-compatible model config section |
| `landing/components/Footer.tsx` | Links, license, integrations |

---

## Task 1: Scaffold the Next.js project

**Files:**
- Create: `landing/package.json`
- Create: `landing/next.config.ts`
- Create: `landing/tsconfig.json`
- Create: `landing/tailwind.config.ts`
- Create: `landing/postcss.config.js`
- Create: `landing/app/globals.css`
- Create: `landing/app/layout.tsx`
- Create: `landing/app/page.tsx`

- [ ] **Step 1: Create `landing/package.json`**

```json
{
  "name": "forge-landing",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start"
  },
  "dependencies": {
    "next": "14.2.3",
    "react": "^18",
    "react-dom": "^18",
    "lucide-react": "^0.376.0"
  },
  "devDependencies": {
    "@types/node": "^20",
    "@types/react": "^18",
    "@types/react-dom": "^18",
    "autoprefixer": "^10.4.19",
    "postcss": "^8",
    "tailwindcss": "^3.4.3",
    "tailwindcss-animate": "^1.0.7",
    "typescript": "^5"
  }
}
```

- [ ] **Step 2: Create `landing/next.config.ts`**

```ts
import type { NextConfig } from "next";

const nextConfig: NextConfig = {};

export default nextConfig;
```

- [ ] **Step 3: Create `landing/tsconfig.json`**

```json
{
  "compilerOptions": {
    "lib": ["dom", "dom.iterable", "esnext"],
    "allowJs": true,
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "jsx": "preserve",
    "incremental": true,
    "plugins": [{ "name": "next" }],
    "paths": { "@/*": ["./*"] }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules"]
}
```

- [ ] **Step 4: Create `landing/postcss.config.js`**

```js
module.exports = {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

- [ ] **Step 5: Create `landing/tailwind.config.ts`**

```ts
import type { Config } from "tailwindcss";
import animate from "tailwindcss-animate";

const config: Config = {
  content: [
    "./app/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./hooks/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        background: "#0d0d0d",
        "accent-blue": "#3b82f6",
        "accent-purple": "#7c3aed",
        muted: "#a1a1aa",
        "code-bg": "#111827",
      },
      fontFamily: {
        sans: ["var(--font-inter)", "sans-serif"],
        mono: ["var(--font-jetbrains)", "monospace"],
      },
      keyframes: {
        "glow-pulse": {
          "0%, 100%": { opacity: "0.3" },
          "50%": { opacity: "0.6" },
        },
        "fade-up": {
          "0%": { opacity: "0", transform: "translateY(24px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        shimmer: {
          "0%": { backgroundPosition: "200% center" },
          "100%": { backgroundPosition: "-200% center" },
        },
      },
      animation: {
        "glow-pulse": "glow-pulse 4s ease-in-out infinite",
        "fade-up": "fade-up 0.6s ease-out forwards",
        shimmer: "shimmer 3s linear infinite",
      },
    },
  },
  plugins: [animate],
};

export default config;
```

- [ ] **Step 6: Create `landing/app/globals.css`**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

html {
  scroll-behavior: smooth;
}

body {
  background-color: #0d0d0d;
  color: white;
}

/* Scrollbar */
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: #0d0d0d; }
::-webkit-scrollbar-thumb { background: #3b82f6; border-radius: 3px; }
```

- [ ] **Step 7: Create `landing/app/layout.tsx`** (empty shell, will be fleshed out in Task 2)

```tsx
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
```

- [ ] **Step 8: Create `landing/app/page.tsx`** (empty shell)

```tsx
export default function Home() {
  return <main>Forge Landing</main>;
}
```

- [ ] **Step 9: Install dependencies**

```bash
cd landing && npm install
```

Expected: `node_modules/` created, no errors.

- [ ] **Step 10: Verify dev server starts**

```bash
cd landing && npm run dev
```

Expected: `ready - started server on 0.0.0.0:3000`. Open browser to `http://localhost:3000` — should show "Forge Landing" text on a white page.

- [ ] **Step 11: Commit**

```bash
cd landing && git add -A && cd .. && git add landing/ && git commit -m "feat(landing): scaffold Next.js 14 project with Tailwind"
```

---

## Task 2: Root layout with fonts and metadata

**Files:**
- Modify: `landing/app/layout.tsx`

- [ ] **Step 1: Replace `landing/app/layout.tsx` with full implementation**

```tsx
import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { JetBrains_Mono } from "next/font/google";
import "./globals.css";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Forge — Autonomous AI Software Engineer",
  description:
    "Forge takes a GitHub issue, spins up an isolated Docker sandbox, autonomously writes and tests code, and produces a verified git diff ready to merge.",
  openGraph: {
    title: "Forge — Autonomous AI Software Engineer",
    description:
      "Autonomous AI agent that fixes GitHub issues end-to-end. Works with any OpenAI-compatible model.",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${inter.variable} ${jetbrainsMono.variable}`}>
      <body className="bg-background font-sans antialiased">{children}</body>
    </html>
  );
}
```

- [ ] **Step 2: Verify fonts load**

Run `npm run dev`, open `http://localhost:3000`, inspect element — body should have `font-family: Inter`.

- [ ] **Step 3: Commit**

```bash
git add landing/app/layout.tsx && git commit -m "feat(landing): add fonts and metadata to root layout"
```

---

## Task 3: `useInView` scroll-reveal hook

**Files:**
- Create: `landing/hooks/useInView.ts`

- [ ] **Step 1: Create `landing/hooks/useInView.ts`**

```ts
"use client";

import { useEffect, useRef, useState } from "react";

export function useInView(threshold = 0.15) {
  const ref = useRef<HTMLDivElement>(null);
  const [inView, setInView] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setInView(true);
          observer.disconnect(); // animate once
        }
      },
      { threshold }
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [threshold]);

  return { ref, inView };
}
```

- [ ] **Step 2: Commit**

```bash
git add landing/hooks/useInView.ts && git commit -m "feat(landing): add useInView scroll-reveal hook"
```

---

## Task 4: Terminal demo data

**Files:**
- Create: `landing/lib/terminal-lines.ts`

- [ ] **Step 1: Create `landing/lib/terminal-lines.ts`**

```ts
export interface TerminalLine {
  type: "cmd" | "output" | "blank";
  text: string;
}

export const TERMINAL_LINES: TerminalLine[] = [
  { type: "cmd",    text: "forge run --github-url https://github.com/owner/repo/issues/25" },
  { type: "output", text: "→ Fetching issue #25 from GitHub..." },
  { type: "output", text: "→ Starting Docker sandbox (forge-sandbox:latest)" },
  { type: "output", text: "→ Cloning repository..." },
  { type: "blank",  text: "" },
  { type: "output", text: "[step 1]  ls -F src" },
  { type: "output", text: "components/  utils/  App.tsx" },
  { type: "output", text: "[step 2]  ls -F src/utils" },
  { type: "output", text: "ls: cannot access 'src/utils': No such file or directory" },
  { type: "output", text: "[step 3]  mkdir -p src/utils" },
  { type: "output", text: "[step 4]  cat << EOF > src/utils/validation.ts ..." },
  { type: "output", text: "[step 5]  cat << EOF > src/utils/string.ts ..." },
  { type: "output", text: "[step 6]  submit" },
  { type: "blank",  text: "" },
  { type: "output", text: "Run complete. Exit status: submitted" },
  { type: "output", text: "diff --git a/src/utils/validation.ts b/src/utils/validation.ts" },
  { type: "output", text: "+++ b/src/utils/validation.ts" },
  { type: "output", text: "+export function isValidEmail(email: string): boolean {" },
  { type: "output", text: "+  const emailRegex = /^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/;" },
  { type: "output", text: "+  return emailRegex.test(email);" },
  { type: "output", text: "+}" },
];
```

- [ ] **Step 2: Commit**

```bash
git add landing/lib/terminal-lines.ts && git commit -m "feat(landing): add terminal demo lines data"
```

---

## Task 5: Navbar component

**Files:**
- Create: `landing/components/Navbar.tsx`

- [ ] **Step 1: Create `landing/components/Navbar.tsx`**

```tsx
"use client";

import { useEffect, useState } from "react";
import { Cpu } from "lucide-react";

export function Navbar() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <nav
      className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
        scrolled
          ? "bg-[#0d0d0d]/90 backdrop-blur-md border-b border-white/5"
          : "bg-transparent"
      }`}
    >
      <div className="max-w-6xl mx-auto px-4 sm:px-6 h-16 flex items-center justify-between">
        {/* Logo */}
        <a href="#" className="flex items-center gap-2 text-white font-bold text-lg">
          <Cpu className="w-5 h-5 text-accent-blue" />
          <span className="bg-gradient-to-r from-accent-blue to-accent-purple bg-clip-text text-transparent">
            Forge
          </span>
        </a>

        {/* Links */}
        <div className="flex items-center gap-4">
          <a
            href="https://github.com"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-muted hover:text-white transition-colors"
          >
            GitHub
          </a>
          <a
            href="#setup"
            className="text-sm px-4 py-2 rounded-lg bg-gradient-to-r from-accent-blue to-accent-purple text-white font-medium hover:opacity-90 transition-opacity"
          >
            Get Started
          </a>
        </div>
      </div>
    </nav>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add landing/components/Navbar.tsx && git commit -m "feat(landing): add sticky Navbar with scroll-aware background"
```

---

## Task 6: Hero section

**Files:**
- Create: `landing/components/Hero.tsx`

- [ ] **Step 1: Create `landing/components/Hero.tsx`**

```tsx
import { ArrowRight, Github } from "lucide-react";

export function Hero() {
  return (
    <section className="relative min-h-screen flex flex-col items-center justify-center text-center px-4 overflow-hidden">
      {/* Background glow */}
      <div
        className="absolute inset-0 pointer-events-none"
        aria-hidden="true"
      >
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] rounded-full bg-gradient-radial from-accent-blue/20 via-accent-purple/10 to-transparent animate-glow-pulse" />
      </div>

      {/* Badge */}
      <div className="relative mb-6 inline-flex items-center gap-2 px-3 py-1 rounded-full border border-accent-blue/30 bg-accent-blue/5 text-accent-blue text-xs font-mono">
        <span className="w-1.5 h-1.5 rounded-full bg-accent-blue animate-pulse" />
        Autonomous AI Agent · Written in Rust
      </div>

      {/* Headline */}
      <h1 className="relative text-5xl sm:text-6xl lg:text-7xl font-bold tracking-tight leading-tight max-w-4xl">
        Your autonomous{" "}
        <span className="bg-gradient-to-r from-accent-blue to-accent-purple bg-clip-text text-transparent">
          AI software engineer
        </span>
      </h1>

      {/* Subheading */}
      <p className="relative mt-6 text-lg sm:text-xl text-muted max-w-2xl leading-relaxed">
        Forge takes a GitHub issue, spins up an isolated Docker sandbox,
        autonomously writes and tests code, and produces a verified{" "}
        <code className="font-mono text-sm bg-code-bg px-1.5 py-0.5 rounded text-accent-blue">
          git diff
        </code>{" "}
        ready to merge.
      </p>

      {/* CTAs */}
      <div className="relative mt-10 flex flex-col sm:flex-row items-center gap-4">
        <a
          href="#setup"
          className="group flex items-center gap-2 px-6 py-3 rounded-xl bg-gradient-to-r from-accent-blue to-accent-purple text-white font-semibold hover:opacity-90 transition-opacity"
        >
          Get Started
          <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform" />
        </a>
        <a
          href="https://github.com"
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center gap-2 px-6 py-3 rounded-xl border border-white/10 text-white/80 font-semibold hover:border-white/20 hover:text-white transition-all"
        >
          <Github className="w-4 h-4" />
          View on GitHub
        </a>
      </div>

      {/* Scroll indicator */}
      <div className="absolute bottom-8 left-1/2 -translate-x-1/2 flex flex-col items-center gap-1 text-muted/40">
        <span className="text-xs font-mono">scroll</span>
        <div className="w-px h-8 bg-gradient-to-b from-muted/40 to-transparent" />
      </div>
    </section>
  );
}
```

- [ ] **Step 2: Wire Navbar + Hero into `app/page.tsx` to preview**

```tsx
import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";

export default function Home() {
  return (
    <main>
      <Navbar />
      <Hero />
    </main>
  );
}
```

- [ ] **Step 3: Verify in browser**

Run `npm run dev`. Open `http://localhost:3000`. Should see: dark background, gradient headline, two CTA buttons, animated glow behind headline, sticky navbar.

- [ ] **Step 4: Commit**

```bash
git add landing/components/Hero.tsx landing/app/page.tsx && git commit -m "feat(landing): add Hero section with gradient glow and CTAs"
```

---

## Task 7: Features section

**Files:**
- Create: `landing/components/Features.tsx`

- [ ] **Step 1: Create `landing/components/Features.tsx`**

```tsx
"use client";

import { Box, Cpu, RefreshCw, FileText, Zap, Cloud } from "lucide-react";
import { useInView } from "@/hooks/useInView";

const FEATURES = [
  {
    icon: Box,
    title: "Docker Sandbox",
    description: "Every run is fully isolated — no leftover state, no host pollution.",
  },
  {
    icon: Cpu,
    title: "Any OpenAI-compatible Model",
    description: "Works with OpenAI, Gemini, Anthropic, Ollama, or any compatible endpoint.",
  },
  {
    icon: RefreshCw,
    title: "Autonomous Agent Loop",
    description: "Thinks, acts, observes, repeats until the task is done or the step limit is reached.",
  },
  {
    icon: FileText,
    title: "Full Trajectory Recording",
    description: "Every step, command, and model response saved to a .traj file.",
  },
  {
    icon: Zap,
    title: "ElizaOS Integration",
    description: "Drop-in action plugin for ElizaOS AI agents.",
  },
  {
    icon: Cloud,
    title: "Nosana Deployment",
    description: "Deploy to decentralized GPU compute with a single JSON job definition.",
  },
];

export function Features() {
  const { ref, inView } = useInView();

  return (
    <section id="features" className="py-24 px-4">
      <div className="max-w-6xl mx-auto">
        {/* Heading */}
        <div className="text-center mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold">
            What{" "}
            <span className="bg-gradient-to-r from-accent-blue to-accent-purple bg-clip-text text-transparent">
              Forge
            </span>{" "}
            does
          </h2>
          <p className="mt-4 text-muted max-w-xl mx-auto">
            A complete autonomous engineering pipeline, from issue to patch.
          </p>
        </div>

        {/* Cards */}
        <div
          ref={ref}
          className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6"
        >
          {FEATURES.map((feature, i) => {
            const Icon = feature.icon;
            return (
              <div
                key={feature.title}
                style={{
                  opacity: inView ? 1 : 0,
                  transform: inView ? "translateY(0)" : "translateY(24px)",
                  transition: `opacity 0.5s ease ${i * 0.08}s, transform 0.5s ease ${i * 0.08}s`,
                }}
                className="group p-6 rounded-2xl border border-white/5 bg-white/[0.02] hover:border-accent-blue/30 hover:bg-white/[0.04] transition-all duration-300"
              >
                <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-accent-blue/20 to-accent-purple/20 flex items-center justify-center mb-4">
                  <Icon className="w-5 h-5 text-accent-blue" />
                </div>
                <h3 className="font-semibold text-white mb-2">{feature.title}</h3>
                <p className="text-sm text-muted leading-relaxed">{feature.description}</p>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
```

- [ ] **Step 2: Add Features to `app/page.tsx`**

```tsx
import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { Features } from "@/components/Features";

export default function Home() {
  return (
    <main>
      <Navbar />
      <Hero />
      <Features />
    </main>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add landing/components/Features.tsx landing/app/page.tsx && git commit -m "feat(landing): add Features grid section with scroll reveal"
```

---

## Task 8: HowItWorks section with animated terminal

**Files:**
- Create: `landing/components/HowItWorks.tsx`

- [ ] **Step 1: Create `landing/components/HowItWorks.tsx`**

```tsx
"use client";

import { useEffect, useRef, useState } from "react";
import { TERMINAL_LINES } from "@/lib/terminal-lines";
import { useInView } from "@/hooks/useInView";

const STEPS = [
  "Fetch the problem statement (GitHub issue, text, or file)",
  "Start an isolated Docker sandbox",
  "Clone the repository",
  "Enter the agent loop — think, act, observe",
  "Execute bash commands autonomously",
  "Run submit to capture the git diff",
  "Save the full trajectory to a .traj file",
];

function TerminalAnimation() {
  const [visibleLines, setVisibleLines] = useState<typeof TERMINAL_LINES>([]);
  const [charIdx, setCharIdx] = useState(0);
  const [lineIdx, setLineIdx] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const currentLine = TERMINAL_LINES[lineIdx];
    if (!currentLine) {
      // Loop: reset after a pause
      const t = setTimeout(() => {
        setVisibleLines([]);
        setLineIdx(0);
        setCharIdx(0);
      }, 2500);
      return () => clearTimeout(t);
    }

    if (currentLine.type === "blank") {
      setVisibleLines((prev) => [...prev, currentLine]);
      setLineIdx((l) => l + 1);
      return;
    }

    if (charIdx < currentLine.text.length) {
      const t = setTimeout(() => setCharIdx((c) => c + 1), 18);
      return () => clearTimeout(t);
    } else {
      const t = setTimeout(() => {
        setVisibleLines((prev) => [...prev, currentLine]);
        setLineIdx((l) => l + 1);
        setCharIdx(0);
      }, 80);
      return () => clearTimeout(t);
    }
  }, [lineIdx, charIdx]);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [visibleLines]);

  const currentLine = TERMINAL_LINES[lineIdx];

  return (
    <div className="rounded-xl overflow-hidden border border-white/10 bg-[#0a0a0a] shadow-2xl">
      {/* Terminal title bar */}
      <div className="flex items-center gap-2 px-4 py-3 bg-white/[0.03] border-b border-white/5">
        <span className="w-3 h-3 rounded-full bg-red-500/70" />
        <span className="w-3 h-3 rounded-full bg-yellow-500/70" />
        <span className="w-3 h-3 rounded-full bg-green-500/70" />
        <span className="ml-2 text-xs text-muted font-mono">forge — bash</span>
      </div>
      {/* Terminal body */}
      <div
        ref={containerRef}
        className="h-72 overflow-y-auto p-4 font-mono text-sm leading-relaxed"
      >
        {visibleLines.map((line, i) => (
          <div key={i} className={line.type === "cmd" ? "text-accent-blue" : "text-white/70"}>
            {line.type === "cmd" && <span className="text-accent-purple mr-2">$</span>}
            {line.text}
          </div>
        ))}
        {/* Currently typing line */}
        {currentLine && currentLine.type !== "blank" && (
          <div className={currentLine.type === "cmd" ? "text-accent-blue" : "text-white/70"}>
            {currentLine.type === "cmd" && (
              <span className="text-accent-purple mr-2">$</span>
            )}
            {currentLine.text.slice(0, charIdx)}
            <span className="inline-block w-2 h-4 bg-accent-blue/80 animate-pulse ml-0.5 align-middle" />
          </div>
        )}
      </div>
    </div>
  );
}

export function HowItWorks() {
  const { ref, inView } = useInView();

  return (
    <section id="how-it-works" className="py-24 px-4">
      <div className="max-w-6xl mx-auto">
        <div className="text-center mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold">
            How it{" "}
            <span className="bg-gradient-to-r from-accent-blue to-accent-purple bg-clip-text text-transparent">
              works
            </span>
          </h2>
          <p className="mt-4 text-muted max-w-xl mx-auto">
            Eight autonomous steps from issue to patch, no human in the loop.
          </p>
        </div>

        <div
          ref={ref}
          style={{
            opacity: inView ? 1 : 0,
            transform: inView ? "translateY(0)" : "translateY(24px)",
            transition: "opacity 0.6s ease, transform 0.6s ease",
          }}
          className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center"
        >
          {/* Terminal */}
          <TerminalAnimation />

          {/* Steps */}
          <ol className="space-y-4">
            {STEPS.map((step, i) => (
              <li key={i} className="flex items-start gap-4">
                <span className="flex-shrink-0 w-7 h-7 rounded-full bg-gradient-to-br from-accent-blue to-accent-purple flex items-center justify-center text-xs font-bold text-white">
                  {i + 1}
                </span>
                <span className="text-white/80 leading-relaxed pt-0.5">{step}</span>
              </li>
            ))}
          </ol>
        </div>
      </div>
    </section>
  );
}
```

- [ ] **Step 2: Add HowItWorks to `app/page.tsx`**

```tsx
import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { Features } from "@/components/Features";
import { HowItWorks } from "@/components/HowItWorks";

export default function Home() {
  return (
    <main>
      <Navbar />
      <Hero />
      <Features />
      <HowItWorks />
    </main>
  );
}
```

- [ ] **Step 3: Verify terminal animation in browser**

`http://localhost:3000` — scroll to the "How it works" section. Terminal should type commands character by character, loop after finishing.

- [ ] **Step 4: Commit**

```bash
git add landing/components/HowItWorks.tsx landing/app/page.tsx && git commit -m "feat(landing): add HowItWorks with animated terminal"
```

---

## Task 9: Setup section

**Files:**
- Create: `landing/components/Setup.tsx`

- [ ] **Step 1: Create `landing/components/Setup.tsx`**

```tsx
"use client";

import { useInView } from "@/hooks/useInView";
import { Terminal } from "lucide-react";

function CodeBlock({ code, language = "bash" }: { code: string; language?: string }) {
  return (
    <div className="rounded-lg overflow-hidden border border-white/10 bg-code-bg">
      <div className="flex items-center gap-2 px-4 py-2 bg-white/[0.03] border-b border-white/5">
        <Terminal className="w-3.5 h-3.5 text-muted" />
        <span className="text-xs text-muted font-mono">{language}</span>
      </div>
      <pre className="p-4 text-sm font-mono text-white/80 overflow-x-auto leading-relaxed">
        <code>{code}</code>
      </pre>
    </div>
  );
}

const STEPS = [
  {
    label: "Prerequisites",
    description: "Docker 24+ and Rust 1.82+ must be installed. Build the sandbox image once:",
    code: "docker build -f Dockerfile.sandbox -t forge-sandbox:latest .",
    language: "bash",
  },
  {
    label: "Clone & build",
    description: "Clone the repository and compile the release binary:",
    code: `git clone <repo-url> && cd forge\ncargo build --release -p forge`,
    language: "bash",
  },
  {
    label: "Configure your model",
    description: "Copy .env.example to .env and set your OpenAI-compatible model credentials:",
    code: `cp .env.example .env\n\n# Edit .env:\nFORGE_MODEL=your-model-name\nFORGE_BASE_URL=https://your-provider.example.com/v1/openai\nFORGE_API_KEY=your-api-key`,
    language: ".env",
  },
  {
    label: "Run against a GitHub issue",
    description: "Point Forge at any public GitHub issue and let it work:",
    code: `set -a && source .env && set +a\n\n./target/release/forge run \\\n  --github-url https://github.com/owner/repo/issues/42`,
    language: "bash",
  },
];

export function Setup() {
  const { ref, inView } = useInView();

  return (
    <section id="setup" className="py-24 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="text-center mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold">
            Get{" "}
            <span className="bg-gradient-to-r from-accent-blue to-accent-purple bg-clip-text text-transparent">
              started locally
            </span>
          </h2>
          <p className="mt-4 text-muted max-w-xl mx-auto">
            Four steps from zero to your first autonomous patch.
          </p>
        </div>

        <div ref={ref} className="space-y-10">
          {STEPS.map((step, i) => (
            <div
              key={step.label}
              style={{
                opacity: inView ? 1 : 0,
                transform: inView ? "translateY(0)" : "translateY(20px)",
                transition: `opacity 0.5s ease ${i * 0.1}s, transform 0.5s ease ${i * 0.1}s`,
              }}
              className="flex gap-6"
            >
              {/* Step number */}
              <div className="flex flex-col items-center">
                <span className="w-8 h-8 rounded-full bg-gradient-to-br from-accent-blue to-accent-purple flex items-center justify-center text-sm font-bold text-white flex-shrink-0">
                  {i + 1}
                </span>
                {i < STEPS.length - 1 && (
                  <div className="w-px flex-1 mt-2 bg-gradient-to-b from-accent-blue/30 to-transparent" />
                )}
              </div>
              {/* Content */}
              <div className="flex-1 pb-2">
                <h3 className="font-semibold text-white mb-1">{step.label}</h3>
                <p className="text-sm text-muted mb-3">{step.description}</p>
                <CodeBlock code={step.code} language={step.language} />
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
```

- [ ] **Step 2: Add Setup to `app/page.tsx`**

```tsx
import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { Features } from "@/components/Features";
import { HowItWorks } from "@/components/HowItWorks";
import { Setup } from "@/components/Setup";

export default function Home() {
  return (
    <main>
      <Navbar />
      <Hero />
      <Features />
      <HowItWorks />
      <Setup />
    </main>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add landing/components/Setup.tsx landing/app/page.tsx && git commit -m "feat(landing): add Setup section with numbered steps and code blocks"
```

---

## Task 10: ModelConfig section

**Files:**
- Create: `landing/components/ModelConfig.tsx`

- [ ] **Step 1: Create `landing/components/ModelConfig.tsx`**

```tsx
"use client";

import { useInView } from "@/hooks/useInView";
import { Terminal } from "lucide-react";

function CodeBlock({ code, language = "bash" }: { code: string; language?: string }) {
  return (
    <div className="rounded-lg overflow-hidden border border-white/10 bg-code-bg">
      <div className="flex items-center gap-2 px-4 py-2 bg-white/[0.03] border-b border-white/5">
        <Terminal className="w-3.5 h-3.5 text-muted" />
        <span className="text-xs text-muted font-mono">{language}</span>
      </div>
      <pre className="p-4 text-sm font-mono text-white/80 overflow-x-auto leading-relaxed">
        <code>{code}</code>
      </pre>
    </div>
  );
}

const PROVIDERS = [
  { name: "OpenAI",         base_url: "https://api.openai.com/v1",                           example: "gpt-4o" },
  { name: "Google Gemini",  base_url: "https://generativelanguage.googleapis.com/v1beta/openai", example: "models/gemini-2.0-flash-001" },
  { name: "Ollama (local)", base_url: "http://localhost:11434/v1",                            example: "llama3" },
  { name: "Anthropic proxy",base_url: "https://your-proxy.example.com/v1",                   example: "claude-3-5-sonnet-20241022" },
];

const ENV_VARS = [
  { name: "FORGE_MODEL",    required: true,  description: "Model identifier passed to the API" },
  { name: "FORGE_BASE_URL", required: true,  description: "Base URL of an OpenAI-compatible completions endpoint" },
  { name: "FORGE_API_KEY",  required: true,  description: "API key for the model endpoint" },
  { name: "GITHUB_TOKEN",   required: false, description: "GitHub PAT — raises API rate limit when fetching issues" },
  { name: "RUST_LOG",       required: false, description: "Log filter (e.g. forge=debug). Default: forge=info" },
];

const YAML_EXAMPLE = `agent:
  model_name: models/gemini-2.0-flash-001
  base_url: https://generativelanguage.googleapis.com/v1beta/openai
  api_key: \$FORGE_API_KEY   # read from env — never hard-code
  max_steps: 25
  parser_type: thought_action  # thought_action | action_only | function_calling`;

export function ModelConfig() {
  const { ref, inView } = useInView();

  return (
    <section id="model-config" className="py-24 px-4">
      <div
        ref={ref}
        style={{
          opacity: inView ? 1 : 0,
          transform: inView ? "translateY(0)" : "translateY(24px)",
          transition: "opacity 0.6s ease, transform 0.6s ease",
        }}
        className="max-w-4xl mx-auto"
      >
        <div className="text-center mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold">
            Any{" "}
            <span className="bg-gradient-to-r from-accent-blue to-accent-purple bg-clip-text text-transparent">
              OpenAI-compatible
            </span>{" "}
            model
          </h2>
          <p className="mt-4 text-muted max-w-xl mx-auto">
            Forge is model-agnostic. Point it at any OpenAI-compatible API endpoint.
          </p>
        </div>

        {/* Provider pills */}
        <div className="flex flex-wrap gap-3 justify-center mb-12">
          {PROVIDERS.map((p) => (
            <div
              key={p.name}
              className="px-4 py-2 rounded-full border border-white/10 bg-white/[0.03] text-sm text-white/70"
            >
              {p.name}
            </div>
          ))}
        </div>

        {/* YAML config */}
        <div className="mb-10">
          <h3 className="font-semibold text-white mb-3">YAML config snippet</h3>
          <CodeBlock code={YAML_EXAMPLE} language="yaml" />
          <p className="mt-2 text-xs text-muted">
            CLI flags override YAML values. Pass{" "}
            <code className="font-mono bg-code-bg px-1 rounded">--model</code>,{" "}
            <code className="font-mono bg-code-bg px-1 rounded">--base-url</code>, or{" "}
            <code className="font-mono bg-code-bg px-1 rounded">--api-key</code> after{" "}
            <code className="font-mono bg-code-bg px-1 rounded">--config</code> to override.
          </p>
        </div>

        {/* Env var table */}
        <div>
          <h3 className="font-semibold text-white mb-3">Environment variables</h3>
          <div className="rounded-xl border border-white/10 overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-white/[0.03] border-b border-white/10">
                  <th className="text-left px-4 py-3 text-muted font-medium">Variable</th>
                  <th className="text-left px-4 py-3 text-muted font-medium">Required</th>
                  <th className="text-left px-4 py-3 text-muted font-medium">Description</th>
                </tr>
              </thead>
              <tbody>
                {ENV_VARS.map((v, i) => (
                  <tr
                    key={v.name}
                    className={i < ENV_VARS.length - 1 ? "border-b border-white/5" : ""}
                  >
                    <td className="px-4 py-3 font-mono text-accent-blue">{v.name}</td>
                    <td className="px-4 py-3">
                      <span
                        className={`text-xs px-2 py-0.5 rounded-full ${
                          v.required
                            ? "bg-accent-blue/10 text-accent-blue border border-accent-blue/20"
                            : "bg-white/5 text-muted border border-white/10"
                        }`}
                      >
                        {v.required ? "Yes" : "No"}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-white/60">{v.description}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </section>
  );
}
```

- [ ] **Step 2: Add ModelConfig to `app/page.tsx`**

```tsx
import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { Features } from "@/components/Features";
import { HowItWorks } from "@/components/HowItWorks";
import { Setup } from "@/components/Setup";
import { ModelConfig } from "@/components/ModelConfig";

export default function Home() {
  return (
    <main>
      <Navbar />
      <Hero />
      <Features />
      <HowItWorks />
      <Setup />
      <ModelConfig />
    </main>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add landing/components/ModelConfig.tsx landing/app/page.tsx && git commit -m "feat(landing): add ModelConfig section with provider pills, YAML, and env table"
```

---

## Task 11: Footer

**Files:**
- Create: `landing/components/Footer.tsx`

- [ ] **Step 1: Create `landing/components/Footer.tsx`**

```tsx
import { Cpu, Github } from "lucide-react";

export function Footer() {
  return (
    <footer className="py-12 px-4 border-t border-white/5">
      <div className="max-w-6xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-6 text-sm text-muted">
        {/* Logo + license */}
        <div className="flex items-center gap-3">
          <Cpu className="w-4 h-4 text-accent-blue" />
          <span className="font-semibold text-white">Forge</span>
          <span className="px-2 py-0.5 rounded-full border border-white/10 text-xs">MIT License</span>
        </div>

        {/* Integrations */}
        <div className="flex items-center gap-6 text-xs">
          <a
            href="https://elizaos.com"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-white transition-colors"
          >
            ElizaOS
          </a>
          <a
            href="https://nosana.com"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-white transition-colors"
          >
            Nosana
          </a>
        </div>

        {/* GitHub + copyright */}
        <div className="flex items-center gap-4">
          <a
            href="https://github.com"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-1.5 hover:text-white transition-colors"
          >
            <Github className="w-4 h-4" />
            GitHub
          </a>
          <span>© {new Date().getFullYear()} Forge</span>
        </div>
      </div>
    </footer>
  );
}
```

- [ ] **Step 2: Wire up final `app/page.tsx`**

```tsx
import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { Features } from "@/components/Features";
import { HowItWorks } from "@/components/HowItWorks";
import { Setup } from "@/components/Setup";
import { ModelConfig } from "@/components/ModelConfig";
import { Footer } from "@/components/Footer";

export default function Home() {
  return (
    <main>
      <Navbar />
      <Hero />
      <Features />
      <HowItWorks />
      <Setup />
      <ModelConfig />
      <Footer />
    </main>
  );
}
```

- [ ] **Step 3: Final build check**

```bash
cd landing && npm run build
```

Expected: no TypeScript errors, no build failures. Output shows `.next/` directory created.

- [ ] **Step 4: Commit**

```bash
git add landing/components/Footer.tsx landing/app/page.tsx && git commit -m "feat(landing): add Footer and complete page assembly"
```

---

## Task 12: Add `bg-gradient-radial` utility (Tailwind fix)

Tailwind CSS v3 does not ship `bg-gradient-radial` by default. The Hero glow uses it.

**Files:**
- Modify: `landing/tailwind.config.ts`

- [ ] **Step 1: Add `backgroundImage` extension to `tailwind.config.ts`**

Open `landing/tailwind.config.ts` and add inside `theme.extend`:

```ts
backgroundImage: {
  "gradient-radial": "radial-gradient(var(--tw-gradient-stops))",
},
```

Full updated `theme.extend` block:

```ts
theme: {
  extend: {
    colors: {
      background: "#0d0d0d",
      "accent-blue": "#3b82f6",
      "accent-purple": "#7c3aed",
      muted: "#a1a1aa",
      "code-bg": "#111827",
    },
    fontFamily: {
      sans: ["var(--font-inter)", "sans-serif"],
      mono: ["var(--font-jetbrains)", "monospace"],
    },
    backgroundImage: {
      "gradient-radial": "radial-gradient(var(--tw-gradient-stops))",
    },
    keyframes: {
      "glow-pulse": {
        "0%, 100%": { opacity: "0.3" },
        "50%": { opacity: "0.6" },
      },
      "fade-up": {
        "0%": { opacity: "0", transform: "translateY(24px)" },
        "100%": { opacity: "1", transform: "translateY(0)" },
      },
      shimmer: {
        "0%": { backgroundPosition: "200% center" },
        "100%": { backgroundPosition: "-200% center" },
      },
    },
    animation: {
      "glow-pulse": "glow-pulse 4s ease-in-out infinite",
      "fade-up": "fade-up 0.6s ease-out forwards",
      shimmer: "shimmer 3s linear infinite",
    },
  },
},
```

- [ ] **Step 2: Verify hero glow renders**

`npm run dev` → `http://localhost:3000` — hero background should show a faint blue/purple radial glow.

- [ ] **Step 3: Commit**

```bash
git add landing/tailwind.config.ts && git commit -m "fix(landing): add gradient-radial backgroundImage utility to Tailwind config"
```

---

## Task 13: Section dividers and final polish

**Files:**
- Modify: `landing/app/globals.css`
- Modify: `landing/app/page.tsx`

- [ ] **Step 1: Add section divider styling to `globals.css`**

Append to `landing/app/globals.css`:

```css
/* Subtle horizontal dividers between sections */
section + section {
  border-top: 1px solid rgba(255, 255, 255, 0.04);
}
```

- [ ] **Step 2: Add `pt-16` to `<main>` in `page.tsx` to clear the fixed navbar**

```tsx
export default function Home() {
  return (
    <main className="pt-16">
      <Navbar />
      <Hero />
      <Features />
      <HowItWorks />
      <Setup />
      <ModelConfig />
      <Footer />
    </main>
  );
}
```

- [ ] **Step 3: Run final build**

```bash
cd landing && npm run build
```

Expected: Build completes successfully, 0 errors, 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add landing/app/globals.css landing/app/page.tsx && git commit -m "feat(landing): add section dividers and final polish"
```

---

## Self-Review Checklist

**Spec coverage:**
- [x] Hero section with headline, subheading, CTAs, glow — Task 6
- [x] Features 3-col grid — Task 7
- [x] HowItWorks animated terminal + numbered steps — Task 8
- [x] Local Setup 4-step guide with code blocks — Task 9
- [x] ModelConfig with providers, YAML, env table — Task 10
- [x] Footer with MIT, GitHub, ElizaOS, Nosana — Task 11
- [x] Sticky navbar transparent→solid on scroll — Task 5
- [x] Scroll-reveal animations — Tasks 3, 7, 8, 9, 10
- [x] Dark charcoal + blue/purple accent theme — Tasks 1, 12
- [x] JetBrains Mono + Inter fonts — Task 2
- [x] Responsive (mobile collapses) — all components use sm/lg breakpoints
- [x] `bg-gradient-radial` fix — Task 12

**Type consistency:** `useInView` returns `{ ref, inView }` — used consistently across Features, HowItWorks, Setup, ModelConfig. `TERMINAL_LINES` type `TerminalLine` used in HowItWorks. All component names match imports in `page.tsx`.

**No placeholders found.**
