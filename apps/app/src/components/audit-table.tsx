import { Button, Checkbox, Icon, IconButton, Select, Tag } from "@workspace/ui/taptap";
import type { CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { type AuditSort, PAGE_SIZES } from "~/lib/audit-table";
import { useDate } from "~/lib/format";

export interface AuditEntry {
  id: number;
  actor: string;
  organization?: string | null;
  project?: string | null;
  action: string;
  target: string;
  client?: string | null;
  origin?: string | null;
  result: string;
  detail?: string | null;
  created_at: string;
}

export const HIDEABLE_COLUMNS = ["via", "target", "origin"] as const;
export type HideableColumn = (typeof HIDEABLE_COLUMNS)[number];

interface PillColors {
  background: string;
  color: string;
}

const PILLS: Record<string, PillColors> = {
  ok: { background: "var(--success-100)", color: "var(--success-600)" },
  denied: { background: "var(--danger-100)", color: "var(--danger-600)" },
  conflict: { background: "var(--warning-100)", color: "var(--warning-600)" },
};

const NEUTRAL_PILL: PillColors = { background: "var(--neutral-100)", color: "var(--neutral-600)" };

const HEAD: CSSProperties = {
  padding: "10px 8px",
  background: "var(--neutral-50)",
  borderBottom: "1px solid var(--neutral-200)",
  font: "500 12px/18px var(--font-sans)",
  color: "var(--neutral-600)",
  textAlign: "left",
};

const CELL: CSSProperties = {
  padding: "10px 8px",
  borderBottom: "1px solid var(--neutral-200)",
  font: "400 13px/20px var(--font-sans)",
  color: "var(--neutral-700)",
};

const MONO: CSSProperties = { fontFamily: "var(--font-mono)" };

export interface AuditTableProps {
  entries: AuditEntry[];
  total: number;
  page: number;
  pageSize: number;
  sort: AuditSort;
  hidden: HideableColumn[];
  showProject: boolean;
  selected: number[];
  onSelect: (ids: number[]) => void;
  onSort: (sort: AuditSort) => void;
  onPage: (page: number) => void;
  onPageSize: (size: number) => void;
}

export function AuditTable({
  entries,
  total,
  page,
  pageSize,
  sort,
  hidden,
  showProject,
  selected,
  onSelect,
  onSort,
  onPage,
  onPageSize,
}: AuditTableProps) {
  const { t } = useTranslation();
  const date = useDate();
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const current = Math.min(page, pages);
  const shown = (column: HideableColumn) => !hidden.includes(column);
  const ids = entries.map((entry) => entry.id);
  const allSelected = ids.length > 0 && ids.every((id) => selected.includes(id));
  const someSelected = !allSelected && ids.some((id) => selected.includes(id));
  const columns = 5 + (showProject ? 1 : 0) + HIDEABLE_COLUMNS.filter(shown).length;
  const toggleRow = (id: number) =>
    onSelect(selected.includes(id) ? selected.filter((item) => item !== id) : [...selected, id]);
  const direction = sort === "newest" ? "descending" : "ascending";
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <div
        style={{
          borderRadius: 4,
          boxShadow: "inset 0 0 0 1px var(--neutral-200)",
          overflowX: "auto",
        }}
      >
        <table
          aria-label={t("audit.table")}
          style={{ width: "100%", minWidth: 900, borderCollapse: "collapse" }}
        >
          <thead>
            <tr>
              <th scope="col" style={{ ...HEAD, width: 40, padding: "10px 12px" }}>
                <Checkbox
                  aria-label={t("audit.selectAll")}
                  checked={allSelected}
                  indeterminate={someSelected}
                  onChange={() => onSelect(allSelected ? [] : ids)}
                />
              </th>
              <th scope="col" aria-sort={direction} style={HEAD}>
                <button
                  type="button"
                  onClick={() => onSort(sort === "newest" ? "oldest" : "newest")}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 4,
                    padding: 0,
                    border: "none",
                    background: "transparent",
                    font: "inherit",
                    color: "inherit",
                    cursor: "pointer",
                  }}
                >
                  {t("audit.when")}
                  <Icon name={sort === "newest" ? "TriDown" : "TriUp"} size={14} />
                </button>
              </th>
              <th scope="col" style={HEAD}>
                {t("audit.actor")}
              </th>
              {shown("via") ? (
                <th scope="col" style={HEAD}>
                  {t("audit.via")}
                </th>
              ) : null}
              <th scope="col" style={HEAD}>
                {t("audit.action")}
              </th>
              {showProject ? (
                <th scope="col" style={HEAD}>
                  {t("audit.project")}
                </th>
              ) : null}
              {shown("target") ? (
                <th scope="col" style={HEAD}>
                  {t("audit.target")}
                </th>
              ) : null}
              <th scope="col" style={HEAD}>
                {t("audit.result")}
              </th>
              {shown("origin") ? (
                <th scope="col" style={HEAD}>
                  {t("audit.origin")}
                </th>
              ) : null}
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => {
              const isSelected = selected.includes(entry.id);
              const pill = PILLS[entry.result] ?? NEUTRAL_PILL;
              return (
                <tr
                  key={entry.id}
                  aria-selected={isSelected}
                  style={{ background: isSelected ? "var(--primary-50)" : "transparent" }}
                >
                  <td style={{ ...CELL, padding: "10px 12px" }}>
                    <Checkbox
                      aria-label={t("audit.selectRow")}
                      checked={isSelected}
                      onChange={() => toggleRow(entry.id)}
                    />
                  </td>
                  <td style={{ ...CELL, whiteSpace: "nowrap" }}>
                    <time dateTime={entry.created_at} title={entry.created_at}>
                      {date(entry.created_at)}
                    </time>
                  </td>
                  <td style={CELL}>{entry.actor}</td>
                  {shown("via") ? <td style={CELL}>{entry.client ?? "—"}</td> : null}
                  <td style={{ ...CELL, ...MONO }}>{entry.action}</td>
                  {showProject ? (
                    <td style={CELL}>{entry.project ?? entry.organization ?? ""}</td>
                  ) : null}
                  {shown("target") ? <td style={CELL}>{entry.target}</td> : null}
                  <td style={CELL}>
                    <Tag style={pill}>
                      {t(`audit.results.${entry.result as "ok" | "denied" | "conflict"}`, {
                        defaultValue: entry.result,
                      })}
                    </Tag>
                    {entry.detail ? (
                      <span style={{ marginLeft: 8, color: "var(--neutral-500)" }}>
                        {entry.detail}
                      </span>
                    ) : null}
                  </td>
                  {shown("origin") ? (
                    <td style={{ ...CELL, ...MONO }}>{entry.origin ?? "—"}</td>
                  ) : null}
                </tr>
              );
            })}
            {entries.length === 0 ? (
              <tr>
                <td
                  colSpan={columns}
                  style={{
                    padding: "48px 0",
                    textAlign: "center",
                    font: "400 14px/22px var(--font-sans)",
                    color: "var(--neutral-500)",
                  }}
                >
                  {t("audit.empty")}
                </td>
              </tr>
            ) : null}
          </tbody>
        </table>
      </div>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 16,
          flexWrap: "wrap",
          font: "400 13px/20px var(--font-sans)",
          color: "var(--neutral-500)",
        }}
      >
        <span style={{ flex: 1, minWidth: 180 }}>
          {t("audit.selected", { count: selected.length, total: entries.length })}
        </span>
        <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {t("audit.rowsPerPage")}
          <Select
            aria-label={t("audit.rowsPerPage")}
            width={80}
            options={PAGE_SIZES.map((size) => ({ label: String(size), value: size }))}
            value={pageSize}
            onChange={(next) => onPageSize(Number(next))}
          />
        </span>
        <span style={{ color: "var(--neutral-700)" }}>
          {t("audit.pageOf", { page: current, pages })}
        </span>
        <span style={{ display: "flex", gap: 4 }}>
          <IconButton
            aria-label={t("audit.first")}
            variant="outline"
            shape="square"
            disabled={current <= 1}
            icon={<Icon name="DoubleLeft" size={16} />}
            onClick={() => onPage(1)}
          />
          <IconButton
            aria-label={t("audit.previous")}
            variant="outline"
            shape="square"
            disabled={current <= 1}
            icon={<Icon name="Left" size={16} />}
            onClick={() => onPage(current - 1)}
          />
          <IconButton
            aria-label={t("audit.next")}
            variant="outline"
            shape="square"
            disabled={current >= pages}
            icon={<Icon name="Right" size={16} />}
            onClick={() => onPage(current + 1)}
          />
          <IconButton
            aria-label={t("audit.last")}
            variant="outline"
            shape="square"
            disabled={current >= pages}
            icon={<Icon name="DoubleRight" size={16} />}
            onClick={() => onPage(pages)}
          />
        </span>
      </div>
    </div>
  );
}

export interface ColumnMenuProps {
  hidden: HideableColumn[];
  onChange: (hidden: HideableColumn[]) => void;
  open: boolean;
  onToggleOpen: (open: boolean) => void;
}

export function ColumnMenu({ hidden, onChange, open, onToggleOpen }: ColumnMenuProps) {
  const { t } = useTranslation();
  return (
    <div style={{ position: "relative", zIndex: 20 }}>
      <Button
        variant="outline"
        aria-haspopup="true"
        aria-expanded={open}
        leftIcon={<Icon name="Setting" size={16} />}
        onClick={() => onToggleOpen(!open)}
      >
        {t("audit.view")}
      </Button>
      {open ? (
        <fieldset
          style={{
            position: "absolute",
            top: 40,
            right: 0,
            width: 200,
            boxSizing: "border-box",
            margin: 0,
            border: "none",
            background: "#fff",
            borderRadius: 4,
            boxShadow: "var(--elevation-2)",
            padding: 4,
          }}
        >
          <legend
            style={{
              padding: "6px 8px",
              font: "400 12px/18px var(--font-sans)",
              color: "var(--neutral-500)",
            }}
          >
            {t("audit.toggleColumns")}
          </legend>
          {HIDEABLE_COLUMNS.map((column) => (
            <Checkbox
              key={column}
              checked={!hidden.includes(column)}
              label={t(`audit.${column}`)}
              onChange={(visible) =>
                onChange(visible ? hidden.filter((item) => item !== column) : [...hidden, column])
              }
              style={{
                display: "flex",
                width: "100%",
                boxSizing: "border-box",
                height: 36,
                padding: "0 8px",
                gap: 10,
                color: "var(--neutral-700)",
              }}
            />
          ))}
        </fieldset>
      ) : null}
    </div>
  );
}
