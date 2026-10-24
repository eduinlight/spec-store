export interface Principle {
  number: number;
  title: string;
  body: string;
}

export interface ParsedConstitution {
  title: string | null;
  intro: string;
  principles: Principle[];
}

const START = /^(\d+)\.\s+\*\*(.+?)\*\*\s*(.*)$/;
const TITLE = /^#\s+(.+?)\s*$/;
const ROMAN: [number, string][] = [
  [1000, "M"],
  [900, "CM"],
  [500, "D"],
  [400, "CD"],
  [100, "C"],
  [90, "XC"],
  [50, "L"],
  [40, "XL"],
  [10, "X"],
  [9, "IX"],
  [5, "V"],
  [4, "IV"],
  [1, "I"],
];

export function toRoman(value: number): string {
  let rest = value;
  let result = "";
  for (const [size, symbol] of ROMAN) {
    while (rest >= size) {
      result += symbol;
      rest -= size;
    }
  }
  return result || String(value);
}

const cleanTitle = (title: string): string => title.replace(/[.:]$/, "").trim();
const cleanBody = (body: string): string => body.replace(/^[.:—–-]\s*/, "").trim();

export function parseConstitution(body: string): ParsedConstitution {
  let title: string | null = null;
  const intro: string[] = [];
  const principles: Principle[] = [];
  let open: Principle | null = null;
  for (const raw of body.split("\n")) {
    const line = raw.trim();
    const heading = TITLE.exec(line);
    if (heading && title === null && principles.length === 0) {
      title = heading[1] ?? null;
      continue;
    }
    const start = START.exec(line);
    if (start) {
      open = {
        number: Number(start[1]),
        title: cleanTitle(start[2] ?? ""),
        body: cleanBody(start[3] ?? ""),
      };
      principles.push(open);
      continue;
    }
    if (line === "") {
      open = null;
      continue;
    }
    if (open) {
      open.body = `${open.body} ${line}`.trim();
    } else if (principles.length === 0) {
      intro.push(line);
    }
  }
  return { title, intro: intro.join(" "), principles };
}
