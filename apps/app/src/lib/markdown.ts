export interface Heading {
  level: number;
  text: string;
  id: string;
}

export const slugify = (text: string): string =>
  text
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "");

const FENCE = /^\s*(```|~~~)/;
const HEADING = /^(#{2,3})\s+(.+?)\s*#*\s*$/;
const INLINE_MARKUP = /[*_`~]/g;

export function headingsOf(body: string): Heading[] {
  const found: Heading[] = [];
  const seen = new Set<string>();
  let fenced = false;
  for (const line of body.split("\n")) {
    if (FENCE.test(line)) {
      fenced = !fenced;
      continue;
    }
    if (fenced) continue;
    const match = HEADING.exec(line);
    if (!match) continue;
    const text = (match[2] ?? "").replace(INLINE_MARKUP, "").trim();
    const id = slugify(text);
    if (!id || seen.has(id)) continue;
    seen.add(id);
    found.push({ level: (match[1] ?? "##").length, text, id });
  }
  return found;
}
