import { buttonVariants } from "@workspace/ui/components/button";
import { rememberLanguage } from "@workspace/ui/lib/language";
import { en } from "./content/en";
import { es } from "./content/es";

export type LandingLanguage = "en" | "es";

export const APP_URL = import.meta.env.VITE_APP_URL ?? "https://app-sddvault.eduindev.com";

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
  const other: LandingLanguage = language === "es" ? "en" : "es";
  const features = [text.features.agents, text.features.history, text.features.teams];
  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="mx-auto flex max-w-5xl items-center justify-between px-6 py-5">
        <a href={language === "es" ? "/es" : "/"} className="flex items-center gap-2 font-semibold">
          <img
            src="/logo-horizontal.png"
            alt="SDD Vault"
            width={133}
            height={32}
            className="h-8 w-auto"
          />
        </a>
        <nav className="flex items-center gap-4 text-sm">
          <a href={text.nav.languageHref} hrefLang={other} onClick={() => rememberLanguage(other)}>
            {text.nav.language}
          </a>
          <a
            href={`${APP_URL}/auth/login`}
            className={buttonVariants({ variant: "outline", size: "sm" })}
          >
            {text.nav.signIn}
          </a>
        </nav>
      </header>
      <main>
        <section className="mx-auto max-w-5xl px-6 py-20 text-center">
          <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">{text.hero.headline}</h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">{text.hero.body}</p>
          <div className="mt-10 flex flex-wrap justify-center gap-3">
            <a
              href={`${APP_URL}/auth/login?action=register`}
              className={buttonVariants({ size: "lg" })}
            >
              {text.hero.signUp}
            </a>
            <a
              href={`${APP_URL}/auth/login`}
              className={buttonVariants({ variant: "outline", size: "lg" })}
            >
              {text.hero.signIn}
            </a>
          </div>
        </section>
        <section className="mx-auto max-w-5xl px-6 pb-24">
          <h2 className="sr-only">{text.features.title}</h2>
          <ul className="grid gap-6 sm:grid-cols-3">
            {features.map((feature) => (
              <li key={feature.title} className="rounded-xl border bg-card p-6">
                <h3 className="font-semibold">{feature.title}</h3>
                <p className="mt-2 text-sm text-muted-foreground">{feature.body}</p>
              </li>
            ))}
          </ul>
        </section>
      </main>
      <footer className="border-t py-6 text-center text-sm text-muted-foreground">
        {text.footer.rights}
      </footer>
    </div>
  );
}
