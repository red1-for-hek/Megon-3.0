import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { Features } from "@/components/Features";
import { HowItWorks } from "@/components/HowItWorks";
import { Setup } from "@/components/Setup";
import { ModelConfig } from "@/components/ModelConfig";
import { Footer } from "@/components/Footer";

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
