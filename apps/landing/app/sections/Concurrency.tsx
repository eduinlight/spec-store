import type { CSSProperties } from "react";
import type { LandingCopy } from "../content/types";
import { BODY_LARGE, CONTAINER, Eyebrow, SECTION_TITLE } from "./shared";

const CHIP: CSSProperties = { padding: "6px 10px", borderRadius: 4, minWidth: 0 };
const NEUTRAL_CHIP: CSSProperties = {
  ...CHIP,
  background: "#fff",
  boxShadow: "inset 0 0 0 1px var(--neutral-200)",
};
const OK_CHIP: CSSProperties = {
  ...CHIP,
  background: "var(--primary-100)",
  color: "var(--primary-700)",
};
const CONFLICT_CHIP: CSSProperties = {
  ...CHIP,
  background: "var(--auxiliary-100)",
  color: "var(--auxiliary-700)",
};
const REVISION: CSSProperties = { fontFamily: "var(--font-mono)", color: "var(--neutral-600)" };
const REVISION_NEW: CSSProperties = { fontFamily: "var(--font-mono)", color: "var(--primary-700)" };

export function Concurrency({ text }: { text: LandingCopy }) {
  const { concurrency } = text;
  return (
    <section
      style={{
        ...CONTAINER,
        padding: "48px 24px 96px",
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,420px),1fr))",
        gap: 48,
        alignItems: "center",
      }}
    >
      <div style={{ order: 2, display: "flex", flexDirection: "column", gap: 16, minWidth: 0 }}>
        <Eyebrow>{concurrency.eyebrow}</Eyebrow>
        <h2 style={SECTION_TITLE}>{concurrency.title}</h2>
        <p style={BODY_LARGE}>{concurrency.body}</p>
      </div>
      <div
        style={{
          order: 1,
          background: "var(--neutral-50)",
          borderRadius: 8,
          padding: 24,
          display: "grid",
          gridTemplateColumns: "56px repeat(2,minmax(0,1fr))",
          gap: "10px 12px",
          fontSize: 13,
          lineHeight: "20px",
          alignItems: "center",
          minWidth: 0,
        }}
      >
        <span />
        <span style={{ fontWeight: 500 }}>{concurrency.agentA}</span>
        <span style={{ fontWeight: 500 }}>{concurrency.agentB}</span>
        <span style={REVISION}>r12</span>
        <span style={NEUTRAL_CHIP}>{concurrency.readPlan}</span>
        <span style={NEUTRAL_CHIP}>{concurrency.readPlan}</span>
        <span style={REVISION_NEW}>r13</span>
        <span style={OK_CHIP}>{concurrency.writeOk}</span>
        <span style={CONFLICT_CHIP}>{concurrency.writeConflict}</span>
        <span />
        <span />
        <span style={NEUTRAL_CHIP}>{concurrency.rereadMerge}</span>
        <span style={REVISION_NEW}>r14</span>
        <span />
        <span style={OK_CHIP}>{concurrency.writeRetry}</span>
      </div>
    </section>
  );
}
