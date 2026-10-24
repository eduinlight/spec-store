import { type SyntheticEvent, useState } from "react";
import type { FaqItem } from "../content/types";
import { SECTION_TITLE } from "./shared";

type FaqProps = {
  title: string;
  items: FaqItem[];
};

export function Faq({ title, items }: FaqProps) {
  const [openIndex, setOpenIndex] = useState(0);

  const handleToggle = (index: number) => (event: SyntheticEvent<HTMLDetailsElement>) => {
    const opened = event.currentTarget.open;
    setOpenIndex((current) => {
      if (opened) return index;
      return current === index ? -1 : current;
    });
  };

  return (
    <section
      id="faq"
      style={{ maxWidth: 840, margin: "0 auto", padding: "88px 24px", scrollMarginTop: 60 }}
    >
      <h2 style={{ ...SECTION_TITLE, margin: "0 0 32px" }}>{title}</h2>
      <div style={{ borderTop: "1px solid var(--neutral-200)" }}>
        {items.map((item, index) => (
          <details
            key={item.q}
            open={openIndex === index}
            onToggle={handleToggle(index)}
            className="group"
            style={{ borderBottom: "1px solid var(--neutral-200)" }}
          >
            <summary
              className="flex cursor-pointer list-none items-center gap-4 [&::-webkit-details-marker]:hidden"
              style={{ padding: "18px 0", font: "500 16px/24px var(--font-sans)" }}
            >
              <span style={{ flex: 1 }}>{item.q}</span>
              <span aria-hidden="true" style={{ color: "var(--neutral-600)", fontSize: 20 }}>
                <span className="group-open:hidden">+</span>
                <span className="hidden group-open:inline">−</span>
              </span>
            </summary>
            <p
              style={{
                margin: 0,
                padding: "0 40px 20px 0",
                fontSize: 14,
                lineHeight: "22px",
                color: "var(--neutral-600)",
              }}
            >
              {item.a}
            </p>
          </details>
        ))}
      </div>
    </section>
  );
}
