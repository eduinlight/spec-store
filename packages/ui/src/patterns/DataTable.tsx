import { type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { Button, IconButton } from "../taptap/Button";
import { Checkbox } from "../taptap/Checkbox";
import { Icon } from "../taptap/Icon";
import { Select } from "../taptap/Select";
import { useControlled } from "./use-controlled";
import { useOutsideDismiss } from "./use-outside-dismiss";

export type DataTableDirection = "asc" | "desc";

export interface DataTableSort {
  key: string;
  direction: DataTableDirection;
}

export interface DataTableColumn<Row> {
  key: string;
  title: string;
  render?: (row: Row) => ReactNode;
  accessor?: (row: Row) => string | number | null | undefined;
  compare?: (a: Row, b: Row) => number;
  sortable?: boolean;
  hideable?: boolean;
  width?: number | string;
  align?: "left" | "center" | "right";
}

export interface DataTableProps<Row> {
  columns: ReadonlyArray<DataTableColumn<Row>>;
  data: ReadonlyArray<Row>;
  rowKey: (row: Row) => string;
  label?: string;
  selectable?: boolean;
  selectedKeys?: ReadonlyArray<string>;
  defaultSelectedKeys?: ReadonlyArray<string>;
  onSelectionChange?: (keys: string[]) => void;
  sort?: DataTableSort | null;
  defaultSort?: DataTableSort | null;
  onSortChange?: (sort: DataTableSort | null) => void;
  page?: number;
  defaultPage?: number;
  onPageChange?: (page: number) => void;
  pageSize?: number;
  defaultPageSize?: number;
  pageSizeOptions?: ReadonlyArray<number>;
  onPageSizeChange?: (pageSize: number) => void;
  hiddenKeys?: ReadonlyArray<string>;
  defaultHiddenKeys?: ReadonlyArray<string>;
  onHiddenKeysChange?: (keys: string[]) => void;
  toolbar?: ReactNode;
  emptyText?: ReactNode;
  minWidth?: number;
}

const NO_KEYS: ReadonlyArray<string> = [];
const DEFAULT_PAGE_SIZES: ReadonlyArray<number> = [5, 10, 20];

const readValue = <Row,>(column: DataTableColumn<Row>, row: Row) =>
  column.accessor ? column.accessor(row) : (row as Record<string, unknown>)[column.key];

const compareValues = (a: unknown, b: unknown): number => {
  if (a == null && b == null) {
    return 0;
  }
  if (a == null) {
    return 1;
  }
  if (b == null) {
    return -1;
  }
  if (typeof a === "number" && typeof b === "number") {
    return a - b;
  }
  return String(a).localeCompare(String(b));
};

const sortRows = <Row,>(
  rows: ReadonlyArray<Row>,
  columns: ReadonlyArray<DataTableColumn<Row>>,
  sort: DataTableSort | null,
): Row[] => {
  const column = sort ? columns.find((item) => item.key === sort.key) : undefined;
  if (!sort || !column?.sortable) {
    return [...rows];
  }
  const sign = sort.direction === "asc" ? 1 : -1;
  const compare =
    column.compare ??
    ((a: Row, b: Row) => compareValues(readValue(column, a), readValue(column, b)));
  return [...rows].sort((a, b) => sign * compare(a, b));
};

interface SortHeaderProps {
  title: string;
  direction: DataTableDirection | null;
  onToggle: () => void;
}

function SortHeader({ title, direction, onToggle }: SortHeaderProps) {
  const [hover, setHover] = useState(false);
  return (
    <button
      type="button"
      onClick={onToggle}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 4,
        padding: 0,
        border: "none",
        background: "transparent",
        font: "inherit",
        color: hover ? "var(--neutral-700)" : "inherit",
        cursor: "pointer",
        userSelect: "none",
      }}
    >
      {title}
      <span style={{ display: "inline-flex", opacity: direction ? 1 : 0.4 }}>
        <Icon
          name={direction === "asc" ? "TriUp" : direction === "desc" ? "TriDown" : "Sort"}
          size={14}
        />
      </span>
    </button>
  );
}

interface DataRowProps {
  selected: boolean;
  children: ReactNode;
}

function DataRow({ selected, children }: DataRowProps) {
  const [hover, setHover] = useState(false);
  return (
    <tr
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        background: hover ? "var(--neutral-50)" : selected ? "var(--primary-50)" : "transparent",
      }}
    >
      {children}
    </tr>
  );
}

interface ToggleRowProps {
  title: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

function ToggleRow({ title, checked, onChange }: ToggleRowProps) {
  const [hover, setHover] = useState(false);
  return (
    <div
      role="none"
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{ borderRadius: 2, background: hover ? "var(--neutral-50)" : "transparent" }}
    >
      <Checkbox
        checked={checked}
        onChange={onChange}
        label={title}
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
    </div>
  );
}

export function DataTable<Row>({
  columns,
  data,
  rowKey,
  label,
  selectable = true,
  selectedKeys,
  defaultSelectedKeys = NO_KEYS,
  onSelectionChange,
  sort,
  defaultSort = null,
  onSortChange,
  page,
  defaultPage = 1,
  onPageChange,
  pageSize,
  defaultPageSize = 10,
  pageSizeOptions = DEFAULT_PAGE_SIZES,
  onPageSizeChange,
  hiddenKeys,
  defaultHiddenKeys = NO_KEYS,
  onHiddenKeysChange,
  toolbar,
  emptyText = "No results.",
  minWidth = 900,
}: DataTableProps<Row>) {
  const [selection, setSelection] = useControlled<ReadonlyArray<string>>(
    selectedKeys,
    defaultSelectedKeys,
    onSelectionChange && ((next) => onSelectionChange([...next])),
  );
  const [activeSort, setActiveSort] = useControlled<DataTableSort | null>(
    sort,
    defaultSort,
    onSortChange,
  );
  const [requestedPage, setPage] = useControlled(page, defaultPage, onPageChange);
  const [size, setSize] = useControlled(pageSize, defaultPageSize, onPageSizeChange);
  const [hidden, setHidden] = useControlled<ReadonlyArray<string>>(
    hiddenKeys,
    defaultHiddenKeys,
    onHiddenKeysChange && ((next) => onHiddenKeysChange([...next])),
  );
  const [viewOpen, setViewOpen] = useState(false);
  const viewRef = useRef<HTMLDivElement>(null);
  const viewButtonRef = useRef<HTMLButtonElement>(null);

  const closeView = useCallback(() => setViewOpen(false), []);
  useOutsideDismiss(viewRef, viewOpen, closeView);

  useEffect(() => {
    if (!viewOpen) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setViewOpen(false);
        viewButtonRef.current?.focus();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [viewOpen]);

  const shown = columns.filter((column) => !hidden.includes(column.key));
  const hideable = columns.filter((column) => column.hideable);
  const ordered = sortRows(data, columns, activeSort);
  const pages = Math.max(1, Math.ceil(ordered.length / size));
  const current = Math.min(Math.max(1, requestedPage), pages);
  const pageRows = ordered.slice((current - 1) * size, current * size);
  const pageKeys = pageRows.map(rowKey);
  const dataKeys = new Set(data.map(rowKey));
  const selectedCount = selection.filter((key) => dataKeys.has(key)).length;
  const allOnPage = pageKeys.length > 0 && pageKeys.every((key) => selection.includes(key));
  const someOnPage = !allOnPage && pageKeys.some((key) => selection.includes(key));

  useEffect(() => {
    if (current !== requestedPage) {
      setPage(current);
    }
  }, [current, requestedPage, setPage]);

  const toggleAll = () =>
    setSelection(
      allOnPage
        ? selection.filter((key) => !pageKeys.includes(key))
        : [...selection, ...pageKeys.filter((key) => !selection.includes(key))],
    );

  const toggleRow = (key: string) =>
    setSelection(
      selection.includes(key) ? selection.filter((item) => item !== key) : [...selection, key],
    );

  const toggleSort = (column: DataTableColumn<Row>) => {
    const direction: DataTableDirection =
      activeSort?.key === column.key && activeSort.direction === "asc" ? "desc" : "asc";
    setActiveSort({ key: column.key, direction });
  };

  const toggleColumn = (key: string, visible: boolean) =>
    setHidden(visible ? hidden.filter((item) => item !== key) : [...hidden, key]);

  const changeSize = (next: number) => {
    setSize(next);
    setPage(1);
  };

  const headerCell = {
    padding: "10px 8px",
    background: "var(--neutral-50)",
    borderBottom: "1px solid var(--neutral-200)",
    font: "500 12px/18px var(--font-sans)",
    color: "var(--neutral-600)",
    textAlign: "left" as const,
  };
  const bodyCell = {
    padding: "10px 8px",
    borderBottom: "1px solid var(--neutral-200)",
    font: "400 13px/20px var(--font-sans)",
    color: "var(--neutral-700)",
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      {toolbar || hideable.length > 0 ? (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8, alignItems: "center" }}>
          {toolbar}
          <span style={{ flex: 1 }} />
          {hideable.length > 0 ? (
            <div ref={viewRef} style={{ position: "relative", zIndex: 20 }}>
              <Button
                ref={viewButtonRef}
                variant="outline"
                aria-haspopup="true"
                aria-expanded={viewOpen}
                leftIcon={<Icon name="Setting" size={16} />}
                onClick={() => setViewOpen(!viewOpen)}
              >
                View
              </Button>
              {viewOpen ? (
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
                    Toggle columns
                  </legend>
                  {hideable.map((column) => (
                    <ToggleRow
                      key={column.key}
                      title={column.title}
                      checked={!hidden.includes(column.key)}
                      onChange={(visible) => toggleColumn(column.key, visible)}
                    />
                  ))}
                </fieldset>
              ) : null}
            </div>
          ) : null}
        </div>
      ) : null}
      <div
        style={{
          borderRadius: 4,
          boxShadow: "inset 0 0 0 1px var(--neutral-200)",
          overflowX: "auto",
        }}
      >
        <table
          aria-label={label}
          style={{ width: "100%", minWidth, borderCollapse: "collapse", tableLayout: "auto" }}
        >
          <thead>
            <tr>
              {selectable ? (
                <th scope="col" style={{ ...headerCell, width: 40, padding: "10px 12px" }}>
                  <Checkbox
                    aria-label="Select all rows on this page"
                    checked={allOnPage}
                    indeterminate={someOnPage}
                    onChange={toggleAll}
                  />
                </th>
              ) : null}
              {shown.map((column) => {
                const direction = activeSort?.key === column.key ? activeSort.direction : null;
                return (
                  <th
                    key={column.key}
                    scope="col"
                    aria-sort={
                      column.sortable
                        ? direction === "asc"
                          ? "ascending"
                          : direction === "desc"
                            ? "descending"
                            : "none"
                        : undefined
                    }
                    style={{
                      ...headerCell,
                      width: column.width,
                      textAlign: column.align ?? "left",
                    }}
                  >
                    {column.sortable ? (
                      <SortHeader
                        title={column.title}
                        direction={direction}
                        onToggle={() => toggleSort(column)}
                      />
                    ) : (
                      column.title
                    )}
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {pageRows.map((row) => {
              const key = rowKey(row);
              const isSelected = selection.includes(key);
              return (
                <DataRow key={key} selected={isSelected}>
                  {selectable ? (
                    <td style={{ ...bodyCell, padding: "10px 12px" }}>
                      <Checkbox
                        aria-label="Select row"
                        checked={isSelected}
                        onChange={() => toggleRow(key)}
                      />
                    </td>
                  ) : null}
                  {shown.map((column) => (
                    <td key={column.key} style={{ ...bodyCell, textAlign: column.align ?? "left" }}>
                      {column.render ? column.render(row) : String(readValue(column, row) ?? "")}
                    </td>
                  ))}
                </DataRow>
              );
            })}
            {pageRows.length === 0 ? (
              <tr>
                <td
                  colSpan={shown.length + (selectable ? 1 : 0)}
                  style={{
                    padding: "48px 0",
                    textAlign: "center",
                    font: "400 14px/22px var(--font-sans)",
                    color: "var(--neutral-500)",
                  }}
                >
                  {emptyText}
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
          {selectable ? `${selectedCount} of ${data.length} row(s) selected.` : null}
        </span>
        <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
          Rows per page
          <Select
            aria-label="Rows per page"
            width={80}
            options={pageSizeOptions.map((option) => ({ label: String(option), value: option }))}
            value={size}
            onChange={(next) => changeSize(Number(next))}
          />
        </span>
        <span style={{ color: "var(--neutral-700)" }}>{`Page ${current} of ${pages}`}</span>
        <span style={{ display: "flex", gap: 4 }}>
          <IconButton
            aria-label="Go to first page"
            variant="outline"
            shape="square"
            disabled={current <= 1}
            icon={<Icon name="DoubleLeft" size={16} />}
            onClick={() => setPage(1)}
          />
          <IconButton
            aria-label="Go to previous page"
            variant="outline"
            shape="square"
            disabled={current <= 1}
            icon={<Icon name="Left" size={16} />}
            onClick={() => setPage(current - 1)}
          />
          <IconButton
            aria-label="Go to next page"
            variant="outline"
            shape="square"
            disabled={current >= pages}
            icon={<Icon name="Right" size={16} />}
            onClick={() => setPage(current + 1)}
          />
          <IconButton
            aria-label="Go to last page"
            variant="outline"
            shape="square"
            disabled={current >= pages}
            icon={<Icon name="DoubleRight" size={16} />}
            onClick={() => setPage(pages)}
          />
        </span>
      </div>
    </div>
  );
}
