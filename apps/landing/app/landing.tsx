import { en } from "./content/en";
import { es } from "./content/es";
import { APP_URL } from "./links";
import { Closing, Footer } from "./sections/Closing";
import { Concurrency } from "./sections/Concurrency";
import { Faq } from "./sections/Faq";
import { Features } from "./sections/Features";
import { Guardrails } from "./sections/Guardrails";
import { Header } from "./sections/Header";
import { Hero } from "./sections/Hero";
import { HowItWorks } from "./sections/HowItWorks";
import { Portable } from "./sections/Portable";
import { Pricing } from "./sections/Pricing";
import { Problem } from "./sections/Problem";
import { Teams } from "./sections/Teams";

export type LandingLanguage = "en" | "es";

export { APP_URL };

export function copy(language: LandingLanguage) {
  return language === "es" ? es : en;
}

export function metaFor(language: LandingLanguage) {
  const text = copy(language);
  return [{ title: text.meta.title }, { name: "description", content: text.meta.description }];
}

export function alternates() {
  return [
    { rel: "alternate", hrefLang: "en", href: "/" },
    { rel: "alternate", hrefLang: "es", href: "/es" },
  ];
}

export function Landing({ language }: { language: LandingLanguage }) {
  const text = copy(language);
  return (
    <div style={{ minHeight: "100vh", background: "#fff", color: "var(--neutral-700)" }}>
      <Header text={text} language={language} />
      <main>
        <Hero text={text} />
        <Problem text={text} />
        <HowItWorks text={text} />
        <Features text={text} />
        <Guardrails text={text} />
        <Concurrency text={text} />
        <Teams text={text} />
        <Portable text={text} />
        <Pricing text={text} />
        <Faq title={text.faq.title} items={text.faq.items} />
        <Closing text={text} />
      </main>
      <Footer text={text} />
    </div>
  );
}
