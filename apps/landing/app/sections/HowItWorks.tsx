import type { LandingCopy } from "../content/types";
import { MCP_COMMAND } from "../links";
import { CARD_BODY, CARD_TITLE, CONTAINER, SectionHeading, Terminal } from "./shared";

export function HowItWorks({ text }: { text: LandingCopy }) {
  return (
    <section id="how-it-works" style={{ ...CONTAINER, padding: "64px 24px", scrollMarginTop: 60 }}>
      <SectionHeading eyebrow={text.how.eyebrow} title={text.how.title} />
      <ol
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,240px),1fr))",
          borderTop: "2px solid var(--primary-600)",
          listStyle: "none",
          margin: 0,
          padding: 0,
        }}
      >
        {text.how.steps.map((step, index) => (
          <li
            key={step.title}
            style={{
              padding: "24px 24px 0 0",
              display: "flex",
              flexDirection: "column",
              gap: 10,
            }}
          >
            <span
              style={{
                font: "500 30px/38px var(--font-numeric)",
                color: "var(--primary-600)",
              }}
            >
              {String(index + 1).padStart(2, "0")}
            </span>
            <h3 style={CARD_TITLE}>{step.title}</h3>
            <p style={CARD_BODY}>{step.body}</p>
          </li>
        ))}
      </ol>
      <Terminal
        style={{ marginTop: 40, padding: "16px 20px", font: "400 13px/22px var(--font-mono)" }}
      >
        {MCP_COMMAND}
      </Terminal>
    </section>
  );
}
