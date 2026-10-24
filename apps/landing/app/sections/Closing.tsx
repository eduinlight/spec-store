import type { LandingCopy } from "../content/types";
import { DEMO_HREF, SECURITY_HREF, SIGN_UP_HREF } from "../links";
import { ActionLink, CONTAINER } from "./shared";

export function Closing({ text }: { text: LandingCopy }) {
  const { cta, closing } = text;
  return (
    <section
      style={{
        background: "var(--primary-100)",
        borderTop: "1px solid var(--primary-200)",
      }}
    >
      <div
        style={{
          ...CONTAINER,
          padding: "80px 24px",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 24,
          textAlign: "center",
        }}
      >
        <h2
          style={{
            margin: 0,
            font: "500 clamp(28px, 6vw, 40px)/1.2 var(--font-sans)",
            maxWidth: 720,
            textWrap: "balance",
          }}
        >
          {closing.title}
        </h2>
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap", justifyContent: "center" }}>
          <ActionLink size="large" href={SIGN_UP_HREF}>
            {cta.startFree}
          </ActionLink>
          <ActionLink size="large" variant="outline" href={DEMO_HREF}>
            {cta.bookDemo}
          </ActionLink>
        </div>
      </div>
    </section>
  );
}

export function Footer({ text }: { text: LandingCopy }) {
  const { footer } = text;
  const footerLinks = [
    { href: "#security", label: footer.security },
    { href: "#how-it-works", label: footer.docs },
    { href: SECURITY_HREF, label: footer.dpa },
    { href: SECURITY_HREF, label: footer.disclosure },
  ];
  return (
    <footer style={{ borderTop: "1px solid var(--neutral-200)" }}>
      <div
        style={{
          ...CONTAINER,
          padding: "32px 24px",
          display: "flex",
          gap: 24,
          flexWrap: "wrap",
          alignItems: "center",
          fontSize: 13,
        }}
      >
        <span style={{ font: "700 14px/1 var(--font-wordmark)", color: "var(--neutral-700)" }}>
          SDD Vault
        </span>
        <span style={{ flex: 1 }} />
        <nav
          aria-label={footer.label}
          style={{ display: "flex", gap: "8px 24px", flexWrap: "wrap" }}
        >
          {footerLinks.map((link) => (
            <a
              key={link.label}
              href={link.href}
              className="text-[var(--neutral-600)] hover:text-[var(--primary-500)]"
            >
              {link.label}
            </a>
          ))}
        </nav>
      </div>
    </footer>
  );
}
