import type { LandingCopy } from "../content/types";
import { BODY_LARGE, CONTAINER, Eyebrow, SECTION_TITLE } from "./shared";

export function Teams({ text }: { text: LandingCopy }) {
  const { teams } = text;
  return (
    <section
      id="security"
      style={{ background: "var(--neutral-700)", color: "#fff", scrollMarginTop: 60 }}
    >
      <div
        style={{
          ...CONTAINER,
          padding: "88px 24px",
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,420px),1fr))",
          gap: 48,
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 16, minWidth: 0 }}>
          <Eyebrow color="var(--primary-400)">{teams.eyebrow}</Eyebrow>
          <h2 style={SECTION_TITLE}>{teams.title}</h2>
          <p style={{ ...BODY_LARGE, color: "var(--neutral-300)" }}>{teams.body}</p>
          <div style={{ paddingTop: 8 }}>
            <a
              href="#security"
              className="text-[var(--primary-400)] hover:text-[var(--primary-300)]"
              style={{ fontWeight: 500 }}
            >
              {teams.overview}
            </a>
          </div>
        </div>
        <ul
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,220px),1fr))",
            gap: 1,
            background: "#4B4B4B",
            borderRadius: 4,
            overflow: "hidden",
            listStyle: "none",
            margin: 0,
            padding: 0,
          }}
        >
          {teams.items.map((item) => (
            <li
              key={item.title}
              style={{
                background: "var(--neutral-700)",
                padding: 20,
                display: "flex",
                flexDirection: "column",
                gap: 6,
              }}
            >
              <span style={{ fontWeight: 500 }}>{item.title}</span>
              <span style={{ fontSize: 13, lineHeight: "20px", color: "var(--neutral-300)" }}>
                {item.body}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
