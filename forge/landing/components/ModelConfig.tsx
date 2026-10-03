"use client";

import { useInView } from "@/hooks/useInView";
import { Terminal } from "lucide-react";
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


const ENV_VARS = [
  { name: "FORGE_MODEL",           required: true,  description: "Model identifier passed to the API" },
  { name: "FORGE_BASE_URL",        required: true,  description: "Base URL of an OpenAI-compatible completions endpoint" },
  { name: "FORGE_API_KEY",         required: true,  description: "API key for the model endpoint" },
  { name: "FORGE_REPO",            required: false, description: "GitHub repo for one-shot mode (owner/repo)" },
  { name: "FORGE_ISSUE",           required: false, description: "Issue number for one-shot mode" },
  { name: "FORGE_WATCH_REPO",      required: false, description: "GitHub repo to monitor in watch mode" },
  { name: "FORGE_WATCH_LABEL",     required: false, description: 'Label to watch for (default: "forge")' },
  { name: "FORGE_WATCH_INTERVAL",  required: false, description: "Seconds between polls (default: 60)" },
  { name: "GITHUB_TOKEN",          required: false, description: "GitHub PAT — raises rate limit; required for private repos" },
];

const PROVIDER_EXAMPLES: { name: string; code: string }[] = [
  {
    name: "OpenAI",
    code: `FORGE_MODEL=gpt-4o\nFORGE_BASE_URL=https://api.openai.com/v1\nFORGE_API_KEY=sk-...`,
  },
  {
    name: "Google Gemini",
    code: `FORGE_MODEL=models/gemini-2.0-flash-001\nFORGE_BASE_URL=https://generativelanguage.googleapis.com/v1beta/openai\nFORGE_API_KEY=AIza...`,
  },
  {
    name: "Ollama (local)",
    code: `FORGE_MODEL=llama3\nFORGE_BASE_URL=http://localhost:11434/v1\nFORGE_API_KEY=ollama`,
  },
  {
    name: "Anthropic proxy",
    code: `FORGE_MODEL=claude-3-5-sonnet-20241022\nFORGE_BASE_URL=https://api.anthropic.com/v1\nFORGE_API_KEY=sk-ant-...`,
  },
];

export function ModelConfig() {
  const { ref, inView } = useInView();
  const [activeProvider, setActiveProvider] = useState(0);

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
            Forge is model-agnostic. Three lines in your <code className="font-mono text-sm bg-code-bg px-1.5 py-0.5 rounded text-accent-blue">.env</code> is all it takes.
          </p>
        </div>

        {/* Provider config switcher */}
        <div className="mb-12">
          <div className="flex flex-wrap gap-2 mb-4">
            {PROVIDER_EXAMPLES.map((p, i) => (
              <button
                key={p.name}
                onClick={() => setActiveProvider(i)}
                className={`px-4 py-1.5 rounded-full text-sm font-medium transition-all border ${
                  activeProvider === i
                    ? "bg-gradient-to-r from-accent-blue to-accent-purple text-white border-transparent"
                    : "border-white/10 text-muted hover:text-white hover:border-white/20"
                }`}
              >
                {p.name}
              </button>
            ))}
          </div>
          <CodeBlock code={PROVIDER_EXAMPLES[activeProvider].code} language=".env" />
          <p className="mt-2 text-xs text-muted">
            Copy the relevant block into your <code className="font-mono bg-code-bg px-1 rounded">.env</code> file. These three variables are the only ones required to start.
          </p>
        </div>

        {/* Env var table */}
        <div>
          <h3 className="font-semibold text-white mb-3">All environment variables</h3>
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
                    <td className="px-4 py-3 font-mono text-accent-blue whitespace-nowrap">{v.name}</td>
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
