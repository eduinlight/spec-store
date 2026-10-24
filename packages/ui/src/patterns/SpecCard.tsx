import { type CSSProperties, type MouseEvent, useId } from "react";
import { AvatarGroup } from "../taptap/Avatar";
import { useInteraction } from "../taptap/interaction";
import { StatusTag, type StatusTagStatus } from "../taptap/Tag";
import { Tooltip } from "../taptap/Tooltip";

export type SpecCardAccent = "active" | "draft" | "none";

export interface SpecCardDocument {
  key: string;
  title: string;
  written: boolean;
}

export interface SpecCardLabels {
  liveAgents: (count: number) => string;
  openQuestions: (count: number) => string;
  requirements: (count: number) => string;
  tasks: (done: number, total: number) => string;
  noTasks: string;
  written: string;
  notWritten: string;
  updated: (when: string) => string;
}

export interface SpecCardProps {
  number: string;
  title: string;
  status: StatusTagStatus;
  statusLabel?: string;
  createdLabel: string;
  summary: string;
  slug: string;
  documents: ReadonlyArray<SpecCardDocument>;
  requirementCount: number;
  updatedLabel: string;
  contributors: ReadonlyArray<string>;
  tasksDone: number;
  tasksTotal: number;
  liveAgents?: number;
  openQuestions?: number;
  openQuestionsTip?: string;
  accent?: SpecCardAccent;
  archived?: boolean;
  href?: string;
  onClick?: (event: MouseEvent<HTMLElement>) => void;
  labels?: Partial<SpecCardLabels>;
}

const ACCENT: Record<SpecCardAccent, { bar: string; ring: string }> = {
  active: { bar: "var(--primary-600)", ring: "inset 0 0 0 1px var(--primary-300)" },
  draft: { bar: "var(--warning-600)", ring: "none" },
  none: { bar: "transparent", ring: "none" },
};

const plural = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`;

const ENGLISH: SpecCardLabels = {
  liveAgents: (count) => `${plural(count, "agent", "agents")} live`,
  openQuestions: (count) => plural(count, "open question", "open questions"),
  requirements: (count) => plural(count, "requirement", "requirements"),
  tasks: (done, total) => `${done} / ${total} tasks`,
  noTasks: "No tasks yet",
  written: "written",
  notWritten: "not written yet",
  updated: (when) => `Updated ${when}`,
};

export function SpecCard({
  number,
  title,
  status,
  statusLabel,
  createdLabel,
  summary,
  slug,
  documents,
  requirementCount,
  updatedLabel,
  contributors,
  tasksDone,
  tasksTotal,
  liveAgents = 0,
  openQuestions = 0,
  openQuestionsTip = "Answer these before the spec can be clarified",
  accent = "none",
  archived = false,
  href,
  onClick,
  labels,
}: SpecCardProps) {
  const text: SpecCardLabels = { ...ENGLISH, ...labels };
  const [state, handlers] = useInteraction();
  const id = useId();
  const numberId = `${id}-number`;
  const titleId = `${id}-title`;
  const statusId = `${id}-status`;
  const liveId = `${id}-live`;
  const questionsId = `${id}-questions`;
  const summaryId = `${id}-summary`;
  const hovered = state === "hover" || state === "pressed";
  const percent = tasksTotal > 0 ? Math.round((tasksDone / tasksTotal) * 100) : 0;
  const palette = ACCENT[accent];
  const describedBy = [
    statusId,
    liveAgents > 0 ? liveId : null,
    openQuestions > 0 ? questionsId : null,
    summaryId,
  ]
    .filter(Boolean)
    .join(" ");

  const cardStyle: CSSProperties = {
    position: "relative",
    display: "grid",
    gridTemplateColumns: "64px minmax(0, 1fr) auto",
    gap: 20,
    alignItems: "center",
    width: "100%",
    boxSizing: "border-box",
    margin: 0,
    padding: "18px 24px 18px 20px",
    border: "none",
    outline: "none",
    borderRadius: 4,
    background: "#fff",
    color: "inherit",
    textAlign: "left",
    textDecoration: "none",
    cursor: "pointer",
    font: "400 14px/22px var(--font-sans)",
    transition: "box-shadow .15s",
    boxShadow:
      state === "focus"
        ? `${palette.ring === "none" ? "" : `${palette.ring}, `}var(--focus-ring)`
        : hovered
          ? "var(--elevation-2)"
          : palette.ring,
  };

  const content = (
    <>
      <span
        aria-hidden="true"
        style={{
          position: "absolute",
          left: 0,
          top: 14,
          bottom: 14,
          width: 3,
          borderRadius: "0 2px 2px 0",
          background: palette.bar,
        }}
      />
      <span style={{ display: "flex", flexDirection: "column", alignItems: "flex-start", gap: 2 }}>
        <span
          id={numberId}
          style={{
            font: "500 22px/30px var(--font-mono)",
            color: archived ? "var(--neutral-400)" : "var(--neutral-700)",
          }}
        >
          {number}
        </span>
        <span style={{ font: "400 11px/16px var(--font-sans)", color: "var(--neutral-500)" }}>
          {createdLabel}
        </span>
      </span>
      <span style={{ display: "flex", flexDirection: "column", gap: 6, minWidth: 0 }}>
        <span style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
          <span
            id={titleId}
            style={{
              font: "500 16px/24px var(--font-sans)",
              color: archived ? "var(--neutral-600)" : "var(--neutral-700)",
            }}
          >
            {title}
          </span>
          <StatusTag id={statusId} status={status}>
            {statusLabel}
          </StatusTag>
          {liveAgents > 0 ? (
            <span
              id={liveId}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 6,
                height: 22,
                padding: "0 8px",
                borderRadius: 100,
                background: "var(--success-100)",
                color: "var(--success-700)",
                font: "400 12px/22px var(--font-sans)",
              }}
            >
              <span
                aria-hidden="true"
                style={{
                  width: 6,
                  height: 6,
                  borderRadius: "50%",
                  background: "var(--success-600)",
                }}
              />
              {text.liveAgents(liveAgents)}
            </span>
          ) : null}
          {openQuestions > 0 ? (
            <Tooltip title={openQuestionsTip}>
              <span
                id={questionsId}
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  height: 22,
                  padding: "0 8px",
                  borderRadius: 100,
                  background: "var(--warning-100)",
                  color: "var(--warning-700)",
                  font: "400 12px/22px var(--font-sans)",
                  cursor: "help",
                }}
              >
                {text.openQuestions(openQuestions)}
              </span>
            </Tooltip>
          ) : null}
        </span>
        <span
          id={summaryId}
          style={{
            font: "400 13px/20px var(--font-sans)",
            color: "var(--neutral-600)",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {summary}
        </span>
        <span
          style={{
            display: "flex",
            alignItems: "center",
            gap: "6px 14px",
            flexWrap: "wrap",
            font: "400 12px/18px var(--font-sans)",
            color: "var(--neutral-500)",
          }}
        >
          <span style={{ fontFamily: "var(--font-mono)" }}>{slug}</span>
          <span style={{ display: "flex", gap: 3 }}>
            {documents.map((document) => (
              <Tooltip
                key={document.key}
                title={`${document.title} · ${document.written ? text.written : text.notWritten}`}
              >
                <span
                  role="img"
                  aria-label={`${document.title} · ${document.written ? "written" : "not written yet"}`}
                  style={{
                    width: 18,
                    height: 18,
                    borderRadius: 2,
                    textAlign: "center",
                    font: "500 11px/18px var(--font-mono)",
                    background: document.written
                      ? archived
                        ? "var(--neutral-100)"
                        : "var(--primary-100)"
                      : "transparent",
                    color: document.written
                      ? archived
                        ? "var(--neutral-600)"
                        : "var(--primary-700)"
                      : "var(--neutral-400)",
                    boxShadow: document.written ? "none" : "inset 0 0 0 1px var(--neutral-300)",
                  }}
                >
                  {document.key}
                </span>
              </Tooltip>
            ))}
          </span>
          <span>{text.requirements(requirementCount)}</span>
          <span>{text.updated(updatedLabel)}</span>
        </span>
      </span>
      <span
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "flex-end",
          gap: 8,
          width: 150,
        }}
      >
        <AvatarGroup
          aria-label="Contributors"
          srcs={contributors.map(() => undefined)}
          names={contributors}
          size={24}
          max={4}
        />
        <span style={{ width: "100%", display: "flex", flexDirection: "column", gap: 4 }}>
          <span
            role="progressbar"
            aria-label="Task progress"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent}
            style={{
              display: "block",
              height: 6,
              borderRadius: 100,
              background: "var(--neutral-200)",
              overflow: "hidden",
            }}
          >
            <span
              style={{
                display: "block",
                height: "100%",
                width: `${percent}%`,
                background: archived ? "var(--neutral-400)" : "var(--primary-600)",
              }}
            />
          </span>
          <span
            style={{
              alignSelf: "flex-end",
              font: "400 12px/18px var(--font-numeric)",
              color: "var(--neutral-500)",
            }}
          >
            {tasksTotal > 0 ? text.tasks(tasksDone, tasksTotal) : text.noTasks}
          </span>
        </span>
      </span>
    </>
  );

  if (href) {
    return (
      <a
        href={href}
        aria-labelledby={`${numberId} ${titleId}`}
        aria-describedby={describedBy}
        onClick={onClick}
        {...handlers}
        style={cardStyle}
      >
        {content}
      </a>
    );
  }
  return (
    <button
      type="button"
      aria-labelledby={`${numberId} ${titleId}`}
      aria-describedby={describedBy}
      onClick={onClick}
      {...handlers}
      style={cardStyle}
    >
      {content}
    </button>
  );
}
