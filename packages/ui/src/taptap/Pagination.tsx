import type { CSSProperties, KeyboardEvent, ReactNode } from "react";

export interface PaginationProps {
  current?: number;
  total?: number;
  pageSize?: number;
  totalItems?: number;
  simple?: boolean;
  showJumper?: boolean;
  showTotal?: boolean;
  onChange?: (page: number) => void;
  style?: CSSProperties;
}

type PageEntry = number | "…";

const pagesFor = (current: number, total: number): PageEntry[] => {
  if (total <= 7) {
    return Array.from({ length: total }, (_, index) => index + 1);
  }
  if (current <= 4) {
    return [1, 2, 3, 4, 5, "…", total];
  }
  if (current >= total - 3) {
    return [1, "…", total - 4, total - 3, total - 2, total - 1, total];
  }
  return [1, "…", current - 1, current, current + 1, "…", total];
};

const Chev = ({ dir }: { dir: "left" | "right" }) => (
  <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
    <path
      d={dir === "left" ? "M7.5 2.5 4 6l3.5 3.5" : "M4.5 2.5 8 6 4.5 9.5"}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.2"
    />
  </svg>
);

interface ItemOptions {
  label: string;
  disabled?: boolean;
  selected?: boolean;
  box?: boolean;
  onClick?: () => void;
}

const Item = ({
  children,
  label,
  disabled,
  selected,
  box,
  onClick,
}: ItemOptions & { children: ReactNode }) => (
  <button
    type="button"
    aria-label={label}
    aria-current={selected ? "page" : undefined}
    disabled={disabled}
    onClick={onClick}
    style={{
      minWidth: 28,
      height: 28,
      padding: "0 6px",
      border: "none",
      borderRadius: 2,
      background: selected ? "var(--primary-200)" : box ? "#fff" : "transparent",
      boxShadow: box ? "inset 0 0 0 1px var(--neutral-200)" : "none",
      color: disabled
        ? "var(--neutral-400)"
        : selected
          ? "var(--primary-600)"
          : "var(--neutral-600)",
      font: "400 14px/22px var(--font-sans)",
      cursor: disabled ? "not-allowed" : "pointer",
      display: "inline-flex",
      alignItems: "center",
      justifyContent: "center",
    }}
  >
    {children}
  </button>
);

export const Pagination = ({
  current = 1,
  total = 50,
  pageSize = 10,
  totalItems,
  simple = false,
  showJumper = true,
  showTotal = false,
  onChange,
  style,
}: PaginationProps) => {
  const go = (page: number) => {
    if (page >= 1 && page <= total && page !== current) {
      onChange?.(page);
    }
  };

  const previous = (
    <Item label="Previous page" box disabled={current <= 1} onClick={() => go(current - 1)}>
      <Chev dir="left" />
    </Item>
  );
  const next = (
    <Item label="Next page" box disabled={current >= total} onClick={() => go(current + 1)}>
      <Chev dir="right" />
    </Item>
  );

  if (simple) {
    return (
      <nav
        aria-label="Pagination"
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 8,
          font: "400 14px/22px var(--font-sans)",
          color: "var(--neutral-600)",
          ...style,
        }}
      >
        {previous}
        <span
          style={{
            minWidth: 38,
            height: 28,
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            borderRadius: 2,
            boxShadow: "inset 0 0 0 1px var(--neutral-300)",
            background: "#fff",
          }}
        >
          {current}
        </span>
        <span>/</span>
        <span>{total}</span>
        {next}
      </nav>
    );
  }

  const itemCount = totalItems ?? total * pageSize;

  return (
    <nav
      aria-label="Pagination"
      style={{ display: "flex", alignItems: "center", gap: 8, ...style }}
    >
      {showTotal && (
        <span
          style={{
            flex: 1,
            font: "400 14px/22px var(--font-sans)",
            color: "var(--neutral-500)",
          }}
        >
          {(current - 1) * pageSize + 1}–{Math.min(current * pageSize, itemCount)} of {itemCount}{" "}
          items{" "}
          <span style={{ marginLeft: 8, color: "var(--neutral-600)" }}>{pageSize}/page ⌄</span>
        </span>
      )}
      {previous}
      {pagesFor(current, total).map((page, index) =>
        page === "…" ? (
          <span
            key={`gap-${index === 1 ? "start" : "end"}`}
            aria-hidden="true"
            style={{
              color: "var(--neutral-500)",
              letterSpacing: 2,
              minWidth: 28,
              textAlign: "center",
            }}
          >
            ···
          </span>
        ) : (
          <Item
            key={page}
            label={`Page ${page}`}
            selected={page === current}
            onClick={() => go(page)}
          >
            {page}
          </Item>
        ),
      )}
      {next}
      {showJumper && (
        <span
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 8,
            marginLeft: 4,
            font: "400 14px/22px var(--font-sans)",
            color: "var(--neutral-600)",
          }}
        >
          Go to
          <input
            aria-label="Go to page"
            onKeyDown={(event: KeyboardEvent<HTMLInputElement>) => {
              if (event.key === "Enter") {
                go(Number.parseInt(event.currentTarget.value, 10));
              }
            }}
            style={{
              width: 44,
              height: 28,
              boxSizing: "border-box",
              border: "none",
              borderRadius: 2,
              boxShadow: "inset 0 0 0 1px var(--neutral-300)",
              outline: "none",
              textAlign: "center",
              font: "inherit",
              color: "inherit",
            }}
          />
        </span>
      )}
    </nav>
  );
};
