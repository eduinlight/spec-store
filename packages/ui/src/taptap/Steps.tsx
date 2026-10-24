import type { CSSProperties, ReactNode } from "react";

export type StepStatus = "wait" | "process" | "finish" | "error";

export interface StepsItem {
  title: ReactNode;
  description?: ReactNode;
  status?: StepStatus;
}

export interface StepsProps {
  items?: StepsItem[];
  current?: number;
  direction?: "horizontal" | "vertical";
  status?: StepStatus;
  style?: CSSProperties;
}

const DOT: Record<StepStatus, [string, string]> = {
  finish: ["var(--primary-100)", "var(--primary-600)"],
  error: ["var(--danger-100)", "var(--danger-600)"],
  process: ["var(--primary-600)", "#fff"],
  wait: ["var(--neutral-100)", "var(--neutral-500)"],
};

const VERTICAL_DOT: Record<StepStatus, string> = {
  process: "var(--primary-600)",
  finish: "var(--primary-300)",
  error: "var(--danger-600)",
  wait: "var(--neutral-400)",
};

const LIST_RESET: CSSProperties = { listStyle: "none", margin: 0, padding: 0 };

const Check = () => (
  <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
    <path d="M2.5 6.2 5 8.5l4.5-5" fill="none" stroke="currentColor" strokeWidth="1.2" />
  </svg>
);

const Cross = () => (
  <svg width="10" height="10" viewBox="0 0 12.5 12.5" aria-hidden="true">
    <path
      fill="currentColor"
      d="M5.421 6.246 0 11.667l.825.825 5.421-5.421 5.421 5.421.825-.825-5.421-5.421L12.492.825 11.667 0 6.246 5.421.825 0 0 .825z"
    />
  </svg>
);

export const Steps = ({
  items = [],
  current = 0,
  direction = "horizontal",
  status,
  style,
}: StepsProps) => {
  const statusOf = (index: number): StepStatus =>
    items[index]?.status ??
    (index < current ? "finish" : index === current ? (status ?? "process") : "wait");

  const entries = items.map((item, index) => ({ item, index, id: `step-${index}` }));

  if (direction === "vertical") {
    return (
      <ol style={{ ...LIST_RESET, display: "flex", flexDirection: "column", ...style }}>
        {entries.map(({ item, index, id }) => {
          const st = statusOf(index);
          const last = index === items.length - 1;
          return (
            <li
              key={id}
              aria-current={index === current ? "step" : undefined}
              style={{ display: "flex", gap: 10, minHeight: last ? 0 : 68 }}
            >
              <div
                aria-hidden="true"
                style={{
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  width: 8,
                  paddingTop: 7,
                }}
              >
                <span
                  style={{
                    width: 6,
                    height: 6,
                    borderRadius: "50%",
                    background: VERTICAL_DOT[st],
                  }}
                />
                {!last && (
                  <span
                    style={{ flex: 1, width: 1, background: "var(--neutral-300)", margin: "8px 0" }}
                  />
                )}
              </div>
              <div>
                <div
                  style={{ font: "400 14px/22px var(--font-sans)", color: "var(--neutral-600)" }}
                >
                  {item.title}
                </div>
                {item.description && (
                  <div
                    style={{ font: "400 12px/18px var(--font-sans)", color: "var(--neutral-500)" }}
                  >
                    {item.description}
                  </div>
                )}
              </div>
            </li>
          );
        })}
      </ol>
    );
  }

  return (
    <ol style={{ ...LIST_RESET, display: "flex", alignItems: "flex-start", ...style }}>
      {entries.map(({ item, index, id }) => {
        const st = statusOf(index);
        const [bg, fg] = DOT[st];
        const last = index === items.length - 1;
        return (
          <li
            key={id}
            aria-current={index === current ? "step" : undefined}
            style={{
              display: "flex",
              alignItems: "flex-start",
              gap: 8,
              flex: last ? "0 0 auto" : 1,
              minWidth: 0,
            }}
          >
            <span
              style={{
                width: 24,
                height: 24,
                flexShrink: 0,
                borderRadius: "50%",
                background: bg,
                color: fg,
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "center",
                font: "400 12px/18px var(--font-sans)",
              }}
            >
              {st === "finish" ? <Check /> : st === "error" ? <Cross /> : index + 1}
            </span>
            <div style={{ flexShrink: 0 }}>
              <div
                style={{
                  font: "400 14px/24px var(--font-sans)",
                  color: st === "wait" ? "var(--neutral-500)" : "var(--neutral-600)",
                }}
              >
                {item.title}
              </div>
              {item.description && (
                <div
                  style={{ font: "400 12px/18px var(--font-sans)", color: "var(--neutral-500)" }}
                >
                  {item.description}
                </div>
              )}
            </div>
            {!last && (
              <span
                aria-hidden="true"
                style={{
                  flex: 1,
                  height: 1,
                  marginTop: 12,
                  marginRight: 16,
                  background: index < current ? "var(--primary-600)" : "var(--neutral-300)",
                }}
              />
            )}
          </li>
        );
      })}
    </ol>
  );
};
