import { Avatar, StatusTag } from "@workspace/ui/taptap";
import type { LandingCopy } from "../content/types";
import { DEMO_HREF, SIGN_UP_HREF } from "../links";
import { ActionLink, CONTAINER, Terminal } from "./shared";

const ELEVATION = "0 6px 16px rgba(0,0,0,.08),0 2px 4px rgba(0,0,0,.06)";

export function Hero({ text }: { text: LandingCopy }) {
  const { hero, cta } = text;
  return (
    <section
      style={{
        background: "var(--neutral-50)",
        borderBottom: "1px solid var(--neutral-200)",
      }}
    >
      <div
        style={{
          ...CONTAINER,
          padding: "72px 24px 56px",
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,460px),1fr))",
          gap: 48,
          alignItems: "center",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 24, minWidth: 0 }}>
          <span
            style={{
              alignSelf: "flex-start",
              padding: "3px 10px",
              borderRadius: 100,
              background: "var(--primary-100)",
              color: "var(--primary-700)",
              fontSize: 12,
              lineHeight: "18px",
              fontWeight: 500,
            }}
          >
            {hero.badge}
          </span>
          <h1
            style={{
              margin: 0,
              font: "500 clamp(34px, 7vw, 52px)/1.15 var(--font-sans)",
              letterSpacing: "-0.02em",
              textWrap: "balance",
            }}
          >
            {hero.headline}
          </h1>
          <p
            style={{
              margin: 0,
              fontSize: 18,
              lineHeight: "28px",
              color: "var(--neutral-600)",
              maxWidth: 540,
              textWrap: "pretty",
            }}
          >
            {hero.body}
          </p>
          <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
            <ActionLink size="large" href={SIGN_UP_HREF}>
              {cta.startFree}
            </ActionLink>
            <ActionLink size="large" variant="outline" href={DEMO_HREF}>
              {cta.bookDemo}
            </ActionLink>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 10, paddingTop: 8 }}>
            <span style={{ fontSize: 12, color: "var(--neutral-600)" }}>{hero.worksWith}</span>
            <ul
              style={{
                display: "flex",
                flexWrap: "wrap",
                gap: "8px 20px",
                font: "500 14px/22px var(--font-sans)",
                color: "var(--neutral-600)",
                listStyle: "none",
                margin: 0,
                padding: 0,
              }}
            >
              {hero.agents.map((agent) => (
                <li key={agent}>{agent}</li>
              ))}
            </ul>
          </div>
        </div>

        <div style={{ position: "relative", display: "grid", gap: 16, minWidth: 0 }}>
          <Terminal
            title="claude · ~/payments"
            style={{
              padding: "16px 18px",
              font: "400 13px/21px var(--font-mono)",
              boxShadow: ELEVATION,
            }}
          >
            <div>
              <span style={{ color: "var(--primary-400)" }}>›</span> {hero.terminal.prompt}
            </div>
            <div style={{ color: "var(--neutral-500)" }}>
              ● sdd-vault.project_status(project: "payments")
            </div>
            <div
              style={{ paddingLeft: 16, borderLeft: "1px solid #4B4B4B", margin: "6px 0 0 4px" }}
            >
              <div>
                {"active_spec  "}
                <span style={{ color: "#fff" }}>004-diesel-orm</span>{" "}
                <span style={{ color: "var(--success-500)" }}>{hero.terminal.approved}</span>
              </div>
              <div>
                {"tasks        "}
                <span style={{ color: "#fff" }}>12 / 20</span> · {hero.terminal.phase}
              </div>
              <div>
                {"next         "}
                <span style={{ color: "var(--primary-400)" }}>T13 [P]</span> FR-18{" "}
                {hero.terminal.nextTask}
              </div>
              <div>
                {"markers      "}
                <span style={{ color: "var(--warning-500)" }}>{hero.terminal.noMarkers}</span>
              </div>
            </div>
          </Terminal>
          <div
            style={{
              background: "#fff",
              borderRadius: 8,
              padding: 20,
              boxShadow: ELEVATION,
              display: "flex",
              flexDirection: "column",
              gap: 14,
              marginLeft: "clamp(0px,6vw,64px)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
              <span style={{ font: "500 12px/18px var(--font-mono)", color: "var(--neutral-600)" }}>
                004
              </span>
              <span style={{ font: "500 16px/24px var(--font-sans)" }}>{hero.card.title}</span>
              <span style={{ flex: 1 }} />
              <StatusTag status="approved">{hero.card.status}</StatusTag>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  fontSize: 12,
                  color: "var(--neutral-600)",
                }}
              >
                <span>{hero.card.phase}</span>
                <span style={{ fontFamily: "var(--font-numeric)" }}>{hero.card.tasks}</span>
              </div>
              <div
                style={{
                  height: 6,
                  borderRadius: 100,
                  background: "var(--neutral-200)",
                  overflow: "hidden",
                }}
              >
                <div style={{ width: "60%", height: "100%", background: "var(--primary-600)" }} />
              </div>
            </div>
            <div
              style={{
                display: "flex",
                gap: 10,
                alignItems: "center",
                padding: "10px 12px",
                borderRadius: 4,
                background: "var(--neutral-50)",
                fontSize: 13,
                lineHeight: "20px",
                flexWrap: "wrap",
              }}
            >
              <Avatar name="Marco Rossi" size={24} />
              <span style={{ color: "var(--neutral-600)", flex: "1 1 160px" }}>
                <b style={{ fontWeight: 500, color: "var(--neutral-700)" }}>{hero.card.actor}</b> ·{" "}
                {hero.card.action}{" "}
                <span style={{ fontFamily: "var(--font-mono)", color: "var(--neutral-700)" }}>
                  T07
                </span>{" "}
                {hero.card.done} · <span style={{ fontFamily: "var(--font-mono)" }}>a1b2c3d</span>
              </span>
              <span style={{ fontSize: 12, color: "var(--neutral-600)", whiteSpace: "nowrap" }}>
                {hero.card.ago}
              </span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
