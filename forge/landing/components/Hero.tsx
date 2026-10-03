import { ArrowRight, Github } from "lucide-react";

export function Hero() {
  return (
    <section className="relative min-h-screen flex flex-col items-center justify-center text-center px-4 overflow-hidden">
      {/* Background glow */}
      <div className="absolute inset-0 pointer-events-none" aria-hidden="true">
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
          href="https://github.com/OkeyAmy/forge"
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
