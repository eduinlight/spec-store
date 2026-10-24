import { Tag } from "@workspace/ui/taptap";
import type { LandingCopy } from "../content/types";
import { DEMO_HREF, SIGN_UP_HREF } from "../links";
import { ActionLink, CONTAINER, SectionHeading } from "./shared";

const PLAN_LAYOUT = [
  { href: SIGN_UP_HREF, variant: "outline", featured: false },
  { href: SIGN_UP_HREF, variant: "outline", featured: false },
  { href: SIGN_UP_HREF, variant: "primary", featured: true },
  { href: DEMO_HREF, variant: "outline", featured: false },
] as const;

export function Pricing({ text }: { text: LandingCopy }) {
  const { pricing } = text;
  return (
    <section id="pricing" style={{ background: "var(--neutral-100)", scrollMarginTop: 60 }}>
      <div style={{ ...CONTAINER, padding: "88px 24px" }}>
        <SectionHeading eyebrow={pricing.eyebrow} title={pricing.title} />
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,250px),1fr))",
            gap: 16,
          }}
        >
          {pricing.plans.map((plan, index) => {
            const layout = PLAN_LAYOUT[index] ?? PLAN_LAYOUT[0];
            return (
              <div
                key={plan.name}
                style={{
                  background: "#fff",
                  borderRadius: 4,
                  padding: 24,
                  display: "flex",
                  flexDirection: "column",
                  gap: 16,
                  boxShadow: layout.featured ? "inset 0 0 0 2px var(--primary-600)" : "none",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                  <h3 style={{ margin: 0, font: "500 20px/28px var(--font-sans)" }}>{plan.name}</h3>
                  {layout.featured ? <Tag type="primary">{pricing.featured}</Tag> : null}
                </div>
                <div style={{ font: "500 24px/32px var(--font-numeric)" }}>{plan.price}</div>
                <span
                  style={{
                    fontSize: 13,
                    lineHeight: "20px",
                    color: "var(--neutral-600)",
                    minHeight: 40,
                  }}
                >
                  {plan.who}
                </span>
                <ul
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: 8,
                    flex: 1,
                    listStyle: "none",
                    margin: 0,
                    padding: 0,
                  }}
                >
                  {plan.items.map((item) => (
                    <li
                      key={item}
                      style={{
                        display: "flex",
                        gap: 8,
                        fontSize: 14,
                        lineHeight: "22px",
                        color: "var(--neutral-600)",
                      }}
                    >
                      <span aria-hidden="true" style={{ color: "var(--primary-600)" }}>
                        ✓
                      </span>
                      <span>{item}</span>
                    </li>
                  ))}
                </ul>
                <ActionLink block variant={layout.variant} href={layout.href}>
                  {plan.cta}
                </ActionLink>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
