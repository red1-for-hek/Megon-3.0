"use client";

import { useInView } from "@/hooks/useInView";
import { Terminal, Zap, AlertTriangle, Info } from "lucide-react";
import { useState } from "react";

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

function WarningBox({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex gap-3 p-3 rounded-lg border border-yellow-500/20 bg-yellow-500/5 text-xs text-yellow-300/80 leading-relaxed">
      <AlertTriangle className="w-4 h-4 text-yellow-400/70 flex-shrink-0 mt-0.5" />
      <span>{children}</span>
    </div>
  );
}

function InfoBox({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex gap-3 p-3 rounded-lg border border-accent-blue/20 bg-accent-blue/5 text-xs text-accent-blue/80 leading-relaxed">
      <Info className="w-4 h-4 text-accent-blue/70 flex-shrink-0 mt-0.5" />
      <span>{children}</span>
    </div>
  );
}

const CONSUMER_STEPS = [
  {
    label: "Install Docker",
    description: "That's the only prerequisite. No Rust, no compiling, no cloning.",
    code: `docker --version`,
    language: "bash",
    note: null,
    warning: null,
  },
  {
    label: "Create a .env file",
    description: "Pick your model provider, paste your API key, and optionally add a GitHub token for automatic PR creation.",
    code: `# .env — create this file anywhere and run from that directory

# Google Gemini (recommended)
FORGE_MODEL=models/gemini-2.0-flash-001
FORGE_BASE_URL=https://generativelanguage.googleapis.com/v1beta/openai
FORGE_API_KEY=your-gemini-api-key

# OpenAI
# FORGE_MODEL=gpt-4o
# FORGE_BASE_URL=https://api.openai.com/v1
# FORGE_API_KEY=sk-...

# GitHub token — enables automatic pull request creation after each fix
# Create one at: github.com/settings/tokens  (needs repo scope)
GITHUB_TOKEN=ghp_...

# Find your Docker group GID: getent group docker | cut -d: -f3
DOCKER_GID=132`,
    language: ".env",
    note: "With GITHUB_TOKEN set, Forge automatically opens a pull request after every successful fix.",
    warning: null,
  },
  {
    label: "Run against a GitHub issue",
    description: "Pass the repo and issue number. Forge pulls the image, clones the repo inside a sandbox, works autonomously, and opens a PR when done.",
    code: `docker compose run --rm \\\n  -e FORGE_REPO=owner/repo \\\n  -e FORGE_ISSUE=42 \\\n  akachiokey/forge:latest`,
    language: "bash",
    note: "Forge clones the repo internally — you never need to clone it yourself.",
    warning: null,
  },
  {
    label: "Enable always-on watch mode",
    description: "Start once, fix forever. Add your repo and token, start the watcher, then just label issues on GitHub — Forge does the rest.",
    code: `# Add to .env:\nFORGE_WATCH_REPO=owner/repo\nFORGE_WATCH_LABEL=forge\nFORGE_WATCH_INTERVAL=60\nGITHUB_TOKEN=ghp_...\n\n# Start in the background:\ndocker compose up watch -d`,
    language: "bash",
    note: "On GitHub, add the label 'forge' to any issue. Within 60 seconds Forge picks it up, fixes it, and pushes a branch forge/issue-{N} for you to review and merge.",
    warning: null,
  },
];

const DEV_STEPS = [
  {
    label: "Prerequisites",
    description: "Docker 24+ and Rust 1.82+ required.",
    code: `docker --version && rustc --version`,
    language: "bash",
    note: null,
    warning: null,
  },
  {
    label: "Clone & build",
    description: "Clone the Forge repo and compile the release binary.",
    code: `git clone https://github.com/OkeyAmy/forge && cd forge\ncargo build --release -p forge`,
    language: "bash",
    note: null,
    warning: null,
  },
  {
    label: "Configure credentials",
    description: "Set your model credentials and GitHub token in .env.",
    code: `cp .env.example .env\n\n# Edit .env:\nFORGE_MODEL=models/gemini-2.0-flash-001\nFORGE_BASE_URL=https://generativelanguage.googleapis.com/v1beta/openai\nFORGE_API_KEY=your-api-key\nGITHUB_TOKEN=ghp_...\nDOCKER_GID=132`,
    language: ".env",
    note: null,
    warning: null,
  },
  {
    label: "Run",
    description: "Load credentials and point Forge at any public GitHub issue.",
    code: `set -a && source .env && set +a\n\n./target/release/forge run \\\n  --repo owner/repo \\\n  --issue 42`,
    language: "bash",
    note: null,
    warning: null,
  },
];

export function Setup() {
  const { ref, inView } = useInView();
  const [mode, setMode] = useState<"consumer" | "dev">("consumer");
  const steps = mode === "consumer" ? CONSUMER_STEPS : DEV_STEPS;

  return (
    <section id="setup" className="py-24 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="text-center mb-12">
          <h2 className="text-3xl sm:text-4xl font-bold">
            Get{" "}
            <span className="bg-gradient-to-r from-accent-blue to-accent-purple bg-clip-text text-transparent">
              started
            </span>
          </h2>
          <p className="mt-4 text-muted max-w-xl mx-auto">
            You pick the issue. Forge does the work.
          </p>

          {/* Mode toggle */}
          <div className="mt-8 inline-flex items-center gap-1 p-1 rounded-xl bg-white/[0.04] border border-white/10">
            <button
              onClick={() => setMode("consumer")}
              className={`flex items-center gap-2 px-5 py-2 rounded-lg text-sm font-medium transition-all ${
                mode === "consumer"
                  ? "bg-gradient-to-r from-accent-blue to-accent-purple text-white shadow"
                  : "text-muted hover:text-white"
              }`}
            >
              <Zap className="w-3.5 h-3.5" />
              Docker
            </button>
            <button
              onClick={() => setMode("dev")}
              className={`flex items-center gap-2 px-5 py-2 rounded-lg text-sm font-medium transition-all ${
                mode === "dev"
                  ? "bg-gradient-to-r from-accent-blue to-accent-purple text-white shadow"
                  : "text-muted hover:text-white"
              }`}
            >
              <Terminal className="w-3.5 h-3.5" />
              Build from source
            </button>
          </div>

          {mode === "consumer" && (
            <p className="mt-3 text-xs text-muted/50 font-mono">
              No Rust · No cloning · No compiling · Just Docker + .env
            </p>
          )}
        </div>

        <div ref={ref} className="space-y-8">
          {steps.map((step, i) => (
            <div
              key={`${mode}-${i}`}
              style={{
                opacity: inView ? 1 : 0,
                transform: inView ? "translateY(0)" : "translateY(20px)",
                transition: `opacity 0.5s ease ${i * 0.07}s, transform 0.5s ease ${i * 0.07}s`,
              }}
              className="flex gap-5"
            >
              <div className="flex flex-col items-center">
                <span className="w-8 h-8 rounded-full bg-gradient-to-br from-accent-blue to-accent-purple flex items-center justify-center text-xs font-bold text-white flex-shrink-0">
                  {i + 1}
                </span>
                {i < steps.length - 1 && (
                  <div className="w-px flex-1 mt-2 bg-gradient-to-b from-accent-blue/30 to-transparent" />
                )}
              </div>

              <div className="flex-1 pb-2 min-w-0">
                <h3 className="font-semibold text-white mb-1">{step.label}</h3>
                <p className="text-sm text-muted mb-3">{step.description}</p>
                <CodeBlock code={step.code} language={step.language} />
                {step.note && (
                  <div className="mt-2">
                    <InfoBox>{step.note}</InfoBox>
                  </div>
                )}
                {step.warning && (
                  <div className="mt-2">
                    <WarningBox>{step.warning}</WarningBox>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
