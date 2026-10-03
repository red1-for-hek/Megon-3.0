"use client";

import { Box, Cpu, RefreshCw, FileText, Zap, GitBranch } from "lucide-react";
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
    icon: GitBranch,
    title: "Auto-fix on Label",
    description: "Label any issue 'forge' — Forge picks it up, fixes it, and pushes branch forge/issue-{N} automatically. No commands to run.",
  },
];

export function Features() {
  const { ref, inView } = useInView();

  return (
    <section id="features" className="py-24 px-4">
      <div className="max-w-6xl mx-auto">
        <div className="text-center mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold">
            What{" "}
            <span className="bg-gradient-to-r from-accent-blue to-accent-purple bg-clip-text text-transparent">
              Forge
            </span>{" "}
            does
          </h2>
          <p className="mt-4 text-muted max-w-xl mx-auto">
            A complete autonomous engineering pipeline, from issue to branch.
          </p>
        </div>

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
                className="p-6 rounded-2xl border border-white/5 bg-white/[0.02] hover:border-accent-blue/30 hover:bg-white/[0.04] transition-all duration-300"
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
