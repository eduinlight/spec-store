import type { CSSProperties, ReactNode } from "react";
import type { LandingCopy } from "../content/types";
import { CARD_BODY, CARD_TITLE, CONTAINER, SectionHeading } from "./shared";

const PANEL: CSSProperties = {
  padding: "12px 16px",
  borderRadius: 4,
  background: "var(--neutral-50)",
  display: "flex",
  flexDirection: "column",
  gap: 6,
  fontSize: 12,
  lineHeight: "18px",
  minWidth: 0,
};

const ROW: CSSProperties = { display: "flex", gap: 8 };
const MONO: CSSProperties = { fontFamily: "var(--font-mono)" };
const MUTED: CSSProperties = { color: "var(--neutral-600)" };
const PILL: CSSProperties = { padding: "2px 8px", borderRadius: 100 };
const ARROW: CSSProperties = { color: "var(--neutral-500)" };
const PARALLEL_TAG: CSSProperties = {
  padding: "0 4px",
  borderRadius: 2,
  background: "var(--primary-100)",
  color: "var(--primary-700)",
};

function FeatureCard({
  title,
  body,
  children,
}: {
  title: string;
  body: string;
  children: ReactNode;
}) {
  return (
    <div
      style={{
        background: "#fff",
        borderRadius: 4,
        padding: 24,
        display: "flex",
        flexDirection: "column",
        gap: 16,
        minWidth: 0,
      }}
    >
      {children}
      <h3 style={CARD_TITLE}>{title}</h3>
      <p style={CARD_BODY}>{body}</p>
    </div>
  );
}

export function Features({ text }: { text: LandingCopy }) {
  const { features } = text;
  const [governed, versioned, parallel, aiNative, multiTenant, secure] = features.cards;
  const [draft, clarified, approved, archived] = features.states;
  const [revisionA, revisionB, revisionC] = features.revisions;
  const [owner, editor, reader] = features.roles;
  const pills: { label: string; style: CSSProperties }[] = [
    { label: draft, style: { background: "var(--neutral-200)", color: "var(--neutral-600)" } },
    { label: clarified, style: { background: "var(--info-100)", color: "var(--info-700)" } },
    { label: approved, style: { background: "var(--success-100)", color: "var(--success-700)" } },
    {
      label: archived,
      style: {
        background: "var(--neutral-100)",
        color: "var(--neutral-600)",
        boxShadow: "inset 0 0 0 1px var(--neutral-300)",
      },
    },
  ];
  return (
    <section id="features" style={{ background: "var(--neutral-100)", scrollMarginTop: 60 }}>
      <div style={{ ...CONTAINER, padding: "88px 24px" }}>
        <SectionHeading eyebrow={features.eyebrow} title={features.title} maxWidth={680} />
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,340px),1fr))",
            gap: 16,
          }}
        >
          <FeatureCard {...governed}>
            <div
              style={{
                ...PANEL,
                flexDirection: "row",
                alignItems: "center",
                flexWrap: "wrap",
                padding: 16,
              }}
            >
              {pills.map((pill, index) => (
                <span key={pill.label} style={{ display: "contents" }}>
                  {index > 0 ? <span style={ARROW}>→</span> : null}
                  <span style={{ ...PILL, ...pill.style }}>{pill.label}</span>
                </span>
              ))}
            </div>
          </FeatureCard>

          <FeatureCard {...versioned}>
            <div style={PANEL}>
              <div style={ROW}>
                <span style={{ ...MONO, color: "var(--primary-700)" }}>r14</span>
                <span>{revisionA}</span>
                <span style={{ marginLeft: "auto", ...MUTED }}>Priya · Cursor</span>
              </div>
              <div style={ROW}>
                <span style={{ ...MONO, ...MUTED }}>r13</span>
                <span>{revisionB}</span>
                <span style={{ marginLeft: "auto", ...MUTED }}>Marco · Claude</span>
              </div>
              <div style={ROW}>
                <span style={{ ...MONO, ...MUTED }}>r12</span>
                <span>{revisionC}</span>
                <span style={{ marginLeft: "auto", ...MUTED }}>Dana · Claude</span>
              </div>
            </div>
          </FeatureCard>

          <FeatureCard {...parallel}>
            <div style={PANEL}>
              {[
                { task: "T07", commit: "a1b2c3d" },
                { task: "T08", commit: "9f8e7d6" },
              ].map((row) => (
                <div key={row.task} style={{ ...ROW, alignItems: "center" }}>
                  <span style={{ color: "var(--success-600)" }}>✓</span>
                  <span style={MONO}>{row.task}</span>
                  <span style={PARALLEL_TAG}>P</span>
                  <span style={{ marginLeft: "auto", ...MONO, ...MUTED }}>{row.commit}</span>
                </div>
              ))}
              <div style={MUTED}>{features.parallelNote}</div>
            </div>
          </FeatureCard>

          <FeatureCard {...aiNative}>
            <div
              style={{
                ...PANEL,
                font: "400 12px/18px var(--font-mono)",
                color: "var(--neutral-600)",
                whiteSpace: "pre",
                overflowX: "auto",
              }}
            >
              {`list_tasks(spec: 4, pending: true)
→ { "id": "T13", "parallel": true,
    "frs": ["FR-18"], "done_when": "..." }`}
            </div>
          </FeatureCard>

          <FeatureCard {...multiTenant}>
            <div style={PANEL}>
              <div style={ROW}>
                <span style={MONO}>payments</span>
                <span style={{ marginLeft: "auto", color: "var(--primary-700)" }}>{owner}</span>
              </div>
              <div style={ROW}>
                <span style={MONO}>checkout</span>
                <span style={{ marginLeft: "auto", color: "var(--neutral-600)" }}>{editor}</span>
              </div>
              <div style={ROW}>
                <span style={MONO}>mobile-app</span>
                <span style={{ marginLeft: "auto", ...MUTED }}>{reader}</span>
              </div>
            </div>
          </FeatureCard>

          <FeatureCard {...secure}>
            <div style={PANEL}>
              <div style={{ ...ROW, flexWrap: "wrap" }}>
                <span style={MONO}>cursor-work</span>
                <span style={MUTED}>{features.tokens.readWrite} · payments</span>
                <span style={{ marginLeft: "auto", ...MUTED }}>{features.tokens.expires}</span>
              </div>
              <div style={{ ...ROW, flexWrap: "wrap" }}>
                <span style={MONO}>ci</span>
                <span style={MUTED}>
                  {features.tokens.readOnly} · {features.tokens.org}
                </span>
                <span style={{ marginLeft: "auto", color: "var(--danger-600)" }}>
                  {features.tokens.revoked}
                </span>
              </div>
            </div>
          </FeatureCard>
        </div>
      </div>
    </section>
  );
}
