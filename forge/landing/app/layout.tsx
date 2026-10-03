import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
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
