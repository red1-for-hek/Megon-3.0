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
  "Push branch forge/issue-{N} — review and merge when ready",
];

function TerminalAnimation() {
  const [visibleLines, setVisibleLines] = useState<typeof TERMINAL_LINES>([]);
  const [charIdx, setCharIdx] = useState(0);
  const [lineIdx, setLineIdx] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const currentLine = TERMINAL_LINES[lineIdx];
    if (!currentLine) {
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
      <div className="flex items-center gap-2 px-4 py-3 bg-white/[0.03] border-b border-white/5">
        <span className="w-3 h-3 rounded-full bg-red-500/70" />
        <span className="w-3 h-3 rounded-full bg-yellow-500/70" />
        <span className="w-3 h-3 rounded-full bg-green-500/70" />
        <span className="ml-2 text-xs text-muted font-mono">forge — bash</span>
      </div>
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
            Eight autonomous steps from issue to pull request, no human in the loop.
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
          <TerminalAnimation />

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
