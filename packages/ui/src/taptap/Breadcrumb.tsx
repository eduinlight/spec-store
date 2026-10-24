import { type CSSProperties, Fragment, type MouseEvent, type ReactNode } from "react";

export interface BreadcrumbItem {
  label: ReactNode;
  href?: string;
  onClick?: () => void;
  ellipsis?: boolean;
}

export interface BreadcrumbProps {
  items?: BreadcrumbItem[];
  maxItems?: number;
  onNavigate?: (item: BreadcrumbItem, index: number) => void;
  style?: CSSProperties;
}

const LINK_RESET: CSSProperties = {
  background: "none",
  border: "none",
  padding: 0,
  font: "inherit",
  textDecoration: "none",
  cursor: "pointer",
  color: "var(--neutral-500)",
};

const highlight = (event: { currentTarget: HTMLElement }) => {
  event.currentTarget.style.color = "var(--primary-600)";
};

const restore = (event: { currentTarget: HTMLElement }) => {
  event.currentTarget.style.color = "var(--neutral-500)";
};

export const Breadcrumb = ({ items = [], maxItems, onNavigate, style }: BreadcrumbProps) => {
  const list: BreadcrumbItem[] =
    maxItems && items.length > maxItems
      ? [
          items[0] as BreadcrumbItem,
          { label: "···", ellipsis: true },
          ...items.slice(items.length - (maxItems - 1)),
        ]
      : items;

  const entries = list.map((item, index) => ({ item, index, id: `crumb-${index}` }));

  return (
    <nav aria-label="Breadcrumb">
      <ol
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          listStyle: "none",
          margin: 0,
          padding: 0,
          font: "400 14px/22px var(--font-sans)",
          ...style,
        }}
      >
        {entries.map(({ item, index, id }) => {
          const last = index === list.length - 1;
          const activate = (event: MouseEvent) => {
            if (!item.href) {
              event.preventDefault();
            }
            item.onClick?.();
            onNavigate?.(item, index);
          };
          const hover = {
            onMouseEnter: highlight,
            onMouseLeave: restore,
            onFocus: highlight,
            onBlur: restore,
          };
          return (
            <Fragment key={id}>
              <li style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                {index > 0 && (
                  <span aria-hidden="true" style={{ color: "var(--neutral-500)" }}>
                    /
                  </span>
                )}
                {last ? (
                  <span
                    aria-current="page"
                    style={{ color: "var(--neutral-600)", cursor: "default" }}
                  >
                    {item.label}
                  </span>
                ) : item.ellipsis ? (
                  <span style={{ color: "var(--neutral-500)", cursor: "default" }}>
                    {item.label}
                  </span>
                ) : item.href ? (
                  <a href={item.href} onClick={activate} style={LINK_RESET} {...hover}>
                    {item.label}
                  </a>
                ) : (
                  <button type="button" onClick={activate} style={LINK_RESET} {...hover}>
                    {item.label}
                  </button>
                )}
              </li>
            </Fragment>
          );
        })}
      </ol>
    </nav>
  );
};
