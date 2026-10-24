const MAX_WORDS = 4;
const SLUG_PATTERN = /^[a-z0-9]+(-[a-z0-9]+){0,3}$/;

export const isValidSlug = (slug: string): boolean => SLUG_PATTERN.test(slug);

export function slugify(name: string): string {
  return name
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean)
    .slice(0, MAX_WORDS)
    .join("-");
}
