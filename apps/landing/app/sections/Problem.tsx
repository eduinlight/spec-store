import type { LandingCopy } from "../content/types";
import { CARD_BODY, CARD_TITLE, CONTAINER, SectionHeading } from "./shared";

export function Problem({ text }: { text: LandingCopy }) {
  return (
    <section style={{ ...CONTAINER, padding: "88px 24px 40px" }}>
      <SectionHeading eyebrow={text.problem.eyebrow} title={text.problem.title} />
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,300px),1fr))",
          gap: 16,
        }}
      >
        {text.problem.cards.map((card) => (
          <div
            key={card.title}
            style={{
              padding: 24,
              borderRadius: 4,
              boxShadow: "inset 0 0 0 1px var(--neutral-200)",
              display: "flex",
              flexDirection: "column",
              gap: 10,
            }}
          >
            <span style={{ font: "500 12px/18px var(--font-mono)", color: "var(--auxiliary-700)" }}>
              {card.tag}
            </span>
            <h3 style={CARD_TITLE}>{card.title}</h3>
            <p style={CARD_BODY}>{card.body}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
