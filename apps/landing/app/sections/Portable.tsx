import type { LandingCopy } from "../content/types";
import { EXPORT_COMMAND, IMPORT_COMMAND } from "../links";
import { BODY_LARGE, CONTAINER, Eyebrow, SECTION_TITLE, Terminal } from "./shared";

const OK = <span style={{ color: "var(--success-500)" }}>ok</span>;

export function Portable({ text }: { text: LandingCopy }) {
  const { portable } = text;
  return (
    <section
      style={{
        ...CONTAINER,
        padding: "88px 24px",
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit,minmax(min(100%,420px),1fr))",
        gap: 48,
        alignItems: "center",
      }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 16, minWidth: 0 }}>
        <Eyebrow>{portable.eyebrow}</Eyebrow>
        <h2 style={SECTION_TITLE}>{portable.title}</h2>
        <p style={BODY_LARGE}>{portable.body}</p>
      </div>
      <Terminal style={{ padding: 20, font: "400 13px/22px var(--font-mono)" }}>
        <div>
          <span style={{ color: "var(--primary-400)" }}>$</span> {IMPORT_COMMAND}
        </div>
        <div>
          {"  docs/constitution.md            "}
          {OK}
          {"  v3"}
        </div>
        <div>
          {"  specs/001-checkout-api/         "}
          {OK}
          {`  5 ${portable.terminal.documents}`}
        </div>
        <div>
          {"  specs/004-diesel-orm/           "}
          {OK}
          {`  4 ${portable.terminal.documents}`}
        </div>
        <div style={{ color: "var(--neutral-500)" }}>{`  ${portable.terminal.summary}`}</div>
        <div style={{ marginTop: 12 }}>
          <span style={{ color: "var(--primary-400)" }}>$</span> {EXPORT_COMMAND}
        </div>
      </Terminal>
    </section>
  );
}
