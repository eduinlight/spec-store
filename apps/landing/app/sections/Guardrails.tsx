import type { CSSProperties, ReactNode } from "react";
import type { LandingCopy } from "../content/types";
import { BODY_LARGE, CONTAINER, Eyebrow, SECTION_TITLE } from "./shared";

const STEP_NUMBER: CSSProperties = {
  flex: "none",
  width: 24,
  height: 24,
  borderRadius: "50%",
  background: "var(--neutral-200)",
  font: "500 12px/24px var(--font-numeric)",
  textAlign: "center",
};

function Step({ number, children }: { number: number; children: ReactNode }) {
  return (
    <li style={{ display: "flex", gap: 12, alignItems: "flex-start" }}>
      <span style={STEP_NUMBER}>{number}</span>
      {children}
    </li>
  );
}

export function Guardrails({ text }: { text: LandingCopy }) {
  const { guardrails } = text;
  return (
    <section
      style={{
        ...CONTAINER,
        padding: "96px 24px 48px",
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,420px),1fr))",
        gap: 48,
        alignItems: "center",
      }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 16, minWidth: 0 }}>
        <Eyebrow>{guardrails.eyebrow}</Eyebrow>
        <h2 style={SECTION_TITLE}>{guardrails.title}</h2>
        <p style={BODY_LARGE}>{guardrails.body}</p>
        <p
          style={{ margin: 0, font: "500 16px/24px var(--font-sans)", color: "var(--neutral-700)" }}
        >
          {guardrails.emphasis}
        </p>
      </div>
      <ol
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 12,
          listStyle: "none",
          margin: 0,
          padding: 0,
          minWidth: 0,
        }}
      >
        <Step number={1}>
          <div
            style={{
              flex: 1,
              minWidth: 0,
              background: "var(--neutral-700)",
              color: "#E1E1E1",
              borderRadius: 4,
              padding: "12px 14px",
              font: "400 12px/20px var(--font-mono)",
              overflowX: "auto",
              whiteSpace: "pre",
            }}
          >
            {`put_constitution(project: "payments",
  base_revision: 7, body: "...coverage ≥ 60%...")`}
          </div>
        </Step>
        <Step number={2}>
          <div
            style={{
              flex: 1,
              minWidth: 0,
              background: "var(--auxiliary-100)",
              boxShadow: "inset 0 0 0 1px var(--auxiliary-300)",
              borderRadius: 4,
              padding: "12px 14px",
              display: "flex",
              flexDirection: "column",
              gap: 2,
            }}
          >
            <span style={{ font: "500 13px/20px var(--font-mono)", color: "var(--auxiliary-700)" }}>
              403 authorization_error
            </span>
            <span style={{ fontSize: 13, lineHeight: "20px", color: "var(--neutral-700)" }}>
              {guardrails.refusal}
            </span>
          </div>
        </Step>
        <Step number={3}>
          <div
            style={{
              flex: 1,
              minWidth: 0,
              background: "#fff",
              boxShadow: "inset 0 0 0 1px var(--neutral-200)",
              borderRadius: 4,
              padding: "12px 14px",
              display: "grid",
              gridTemplateColumns: "auto minmax(0,1fr) auto",
              gap: "4px 12px",
              fontSize: 12,
              lineHeight: "18px",
            }}
          >
            <span style={{ color: "var(--neutral-600)" }}>14:06:52</span>
            <span>
              <b style={{ fontWeight: 500 }}>dana</b> · cursor-work ·{" "}
              <span style={{ fontFamily: "var(--font-mono)" }}>constitution.write</span>
            </span>
            <span style={{ color: "var(--auxiliary-700)" }}>{guardrails.audit.denied}</span>
            <span />
            <span style={{ color: "var(--neutral-600)" }}>{guardrails.audit.detail}</span>
            <span />
          </div>
        </Step>
      </ol>
    </section>
  );
}
