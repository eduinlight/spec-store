import { rememberLanguage } from "@workspace/ui/lib/language";
import type { LandingCopy } from "../content/types";
import { DEMO_HREF, SIGN_IN_HREF, SIGN_UP_HREF } from "../links";
import { ActionLink, CONTAINER } from "./shared";

type HeaderProps = {
  text: LandingCopy;
  language: "en" | "es";
};

export function Header({ text, language }: HeaderProps) {
  const other = language === "es" ? "en" : "es";
  const anchors = [
    { href: "#how-it-works", label: text.nav.howItWorks },
    { href: "#features", label: text.nav.features },
    { href: "#security", label: text.nav.security },
    { href: "#pricing", label: text.nav.pricing },
    { href: "#faq", label: text.nav.faq },
  ];
  return (
    <header
      style={{
        position: "sticky",
        top: 0,
        zIndex: 10,
        background: "#fff",
        boxShadow: "0 1px 5px rgba(45,47,51,.1)",
      }}
    >
      <div
        style={{
          ...CONTAINER,
          padding: "8px 24px",
          minHeight: 60,
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: "8px 32px",
        }}
      >
        <a href={language === "es" ? "/es" : "/"} style={{ display: "flex", alignItems: "center" }}>
          <img
            src="/logo-horizontal.png"
            alt={text.nav.logoAlt}
            width={133}
            height={32}
            style={{ display: "block", width: 133, height: 32 }}
          />
        </a>
        <nav
          aria-label={text.nav.label}
          className="hidden min-[880px]:flex"
          style={{ gap: 24, flex: 1, fontSize: 14 }}
        >
          {anchors.map((anchor) => (
            <a
              key={anchor.href}
              href={anchor.href}
              className="text-[var(--neutral-600)] hover:text-[var(--primary-500)]"
            >
              {anchor.label}
            </a>
          ))}
        </nav>
        <div style={{ display: "flex", gap: 8, alignItems: "center", marginLeft: "auto" }}>
          <a
            href={text.nav.languageHref}
            hrefLang={other}
            onClick={() => rememberLanguage(other)}
            className="text-[var(--neutral-600)] hover:text-[var(--primary-500)]"
            style={{ fontSize: 14, padding: "0 8px" }}
          >
            {text.nav.language}
          </a>
          <a
            href={SIGN_IN_HREF}
            className="text-[var(--neutral-600)] hover:text-[var(--primary-500)]"
            style={{ fontSize: 14, padding: "0 8px" }}
          >
            {text.cta.signIn}
          </a>
          <ActionLink variant="outline" href={DEMO_HREF} className="max-sm:hidden">
            {text.cta.bookDemo}
          </ActionLink>
          <ActionLink href={SIGN_UP_HREF}>{text.cta.startFree}</ActionLink>
        </div>
      </div>
    </header>
  );
}
