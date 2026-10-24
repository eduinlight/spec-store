import { type CSSProperties, Fragment, type Key, type ReactNode, useState } from "react";

export interface TableColumn<Row> {
  key: string;
  title: ReactNode;
  width?: number | string;
  maxWidth?: number | string;
  align?: "left" | "center" | "right";
  render?: (value: unknown, row: Row) => ReactNode;
}

export interface TableProps<Row extends object> {
  columns?: TableColumn<Row>[];
  data?: Row[];
  rowKey?: string;
  selectable?: boolean;
  selected?: Key[];
  onSelect?: (selected: Key[]) => void;
  bordered?: boolean;
  expandable?: (row: Row) => ReactNode;
  footer?: ReactNode;
  emptyText?: ReactNode;
  caption?: string;
  style?: CSSProperties;
}

const CELL: CSSProperties = {
  padding: "7px 8px",
  font: "400 12px/22px var(--font-sans)",
  color: "var(--neutral-600)",
  textAlign: "left",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};

const readCell = (row: object, key: string): unknown => (row as Record<string, unknown>)[key];

export function Table<Row extends object>({
  columns = [],
  data = [],
  rowKey = "id",
  selectable = false,
  selected = [],
  onSelect,
  bordered = false,
  expandable,
  footer,
  emptyText = "No data",
  caption,
  style,
}: TableProps<Row>) {
  const [hover, setHover] = useState<number | null>(null);
  const [open, setOpen] = useState<Key[]>([]);
  const keyOf = (row: Row) => readCell(row, rowKey) as Key;
  const allSelected = data.length > 0 && data.every((row) => selected.includes(keyOf(row)));
  const toggleRow = (key: Key) =>
    onSelect?.(
      selected.includes(key) ? selected.filter((item) => item !== key) : [...selected, key],
    );
  const span = columns.length + (expandable ? 1 : 0) + (selectable ? 1 : 0);
  return (
    <div style={{ overflowX: "auto", ...style }}>
      <table style={{ width: "100%", borderCollapse: "collapse", tableLayout: "auto" }}>
        {caption && (
          <caption
            style={{
              position: "absolute",
              width: 1,
              height: 1,
              overflow: "hidden",
              clip: "rect(0 0 0 0)",
            }}
          >
            {caption}
          </caption>
        )}
        <thead>
          <tr style={{ background: "var(--neutral-50)" }}>
            {expandable && (
              <th scope="col" style={{ ...CELL, width: 24 }}>
                <span
                  style={{
                    position: "absolute",
                    width: 1,
                    height: 1,
                    overflow: "hidden",
                    clip: "rect(0 0 0 0)",
                  }}
                >
                  Expand
                </span>
              </th>
            )}
            {selectable && (
              <th scope="col" style={{ ...CELL, width: 24 }}>
                <input
                  type="checkbox"
                  aria-label="Select all rows"
                  checked={allSelected}
                  onChange={() => onSelect?.(allSelected ? [] : data.map(keyOf))}
                />
              </th>
            )}
            {columns.map((column) => (
              <th
                key={column.key}
                scope="col"
                style={{
                  ...CELL,
                  fontWeight: 500,
                  color: "var(--neutral-600)",
                  width: column.width,
                  textAlign: column.align ?? "left",
                  borderRight: bordered ? "1px solid var(--neutral-200)" : "none",
                }}
              >
                {column.title}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.length === 0 && (
            <tr>
              <td
                colSpan={Math.max(span, 1)}
                style={{ ...CELL, padding: "24px 8px", textAlign: "center" }}
              >
                {emptyText}
              </td>
            </tr>
          )}
          {data.map((row, index) => {
            const key = keyOf(row);
            const isOpen = open.includes(key);
            return (
              <Fragment key={key}>
                <tr
                  onMouseEnter={() => setHover(index)}
                  onMouseLeave={() => setHover(null)}
                  style={{
                    background: hover === index ? "var(--neutral-50)" : "#fff",
                    borderBottom: "1px solid var(--neutral-200)",
                  }}
                >
                  {expandable && (
                    <td style={CELL}>
                      <button
                        type="button"
                        aria-label={isOpen ? "Collapse row" : "Expand row"}
                        aria-expanded={isOpen}
                        onClick={() =>
                          setOpen((current) =>
                            isOpen ? current.filter((item) => item !== key) : [...current, key],
                          )
                        }
                        style={{
                          cursor: "pointer",
                          display: "inline-flex",
                          padding: 0,
                          border: 0,
                          background: "none",
                          color: "var(--neutral-500)",
                          transform: isOpen ? "rotate(90deg)" : "none",
                        }}
                      >
                        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
                          <path
                            d="M4.5 2.5 8 6 4.5 9.5"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="1.2"
                          />
                        </svg>
                      </button>
                    </td>
                  )}
                  {selectable && (
                    <td style={CELL}>
                      <input
                        type="checkbox"
                        aria-label="Select row"
                        checked={selected.includes(key)}
                        onChange={() => toggleRow(key)}
                      />
                    </td>
                  )}
                  {columns.map((column) => (
                    <td
                      key={column.key}
                      style={{
                        ...CELL,
                        textAlign: column.align ?? "left",
                        borderRight: bordered ? "1px solid var(--neutral-200)" : "none",
                        maxWidth: column.maxWidth,
                      }}
                    >
                      {column.render
                        ? column.render(readCell(row, column.key), row)
                        : (readCell(row, column.key) as ReactNode)}
                    </td>
                  ))}
                </tr>
                {isOpen && expandable && (
                  <tr
                    style={{
                      background: "var(--neutral-50)",
                      borderBottom: "1px solid var(--neutral-200)",
                    }}
                  >
                    <td colSpan={span} style={{ ...CELL, padding: "8px 40px" }}>
                      {expandable(row)}
                    </td>
                  </tr>
                )}
              </Fragment>
            );
          })}
        </tbody>
        {footer && (
          <tfoot>
            <tr style={{ background: "var(--neutral-50)" }}>{footer}</tr>
          </tfoot>
        )}
      </table>
    </div>
  );
}
