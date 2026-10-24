import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { en } from "./content/en";
import { es } from "./content/es";
import type { LandingCopy } from "./content/types";

const client = resolve(import.meta.dirname, "../build/client");
const app = process.env.VITE_APP_URL ?? "https://app-sddvault.eduindev.com";

const ENTITIES: Record<string, string> = {
  "&amp;": "&",
  "&lt;": "<",
  "&gt;": ">",
  "&quot;": '"',
  "&#x27;": "'",
  "&#39;": "'",
};

const textOf = (html: string) =>
  html
    .replace(/<[^>]*>/g, " ")
    .replace(/&(amp|lt|gt|quot|#x27|#39);/g, (entity) => ENTITIES[entity] ?? entity)
    .replace(/\s+/g, " ");

const countOf = (html: string, fragment: string) => html.split(fragment).length - 1;

const PAGES = [
  {
    language: "en",
    file: "index.html",
    copy: en as LandingCopy,
    headline: "Specs your AI agents follow, and can't rewrite.",
    plans: [
      ["Free", "$0"],
      ["Pro", "Per user"],
      ["Team", "Per seat"],
      ["Enterprise", "Custom"],
    ],
    alternate: "es",
  },
  {
    language: "es",
    file: "es/index.html",
    copy: es as LandingCopy,
    headline: "Specs que tus agentes de IA siguen y no pueden reescribir.",
    plans: [
      ["Free", "$0"],
      ["Pro", "Por usuario"],
      ["Team", "Por puesto"],
      ["Enterprise", "A medida"],
    ],
    alternate: "en",
  },
] as const;

describe.each(PAGES)(
  "the $language page",
  ({ language, file, copy, headline, plans, alternate }) => {
    const html = readFileSync(resolve(client, file), "utf8");
    const text = textOf(html.slice(html.indexOf("<body")));

    test("is complete in the first HTML response with its alternates", () => {
      expect(html).toContain(`<html lang="${language}"`);
      expect(html).toMatch(new RegExp(`hreflang="${alternate}"`, "i"));
      expect(textOf(html.slice(0, html.indexOf("</title>")))).toContain(copy.meta.title);
    });

    test("shows the hero headline", () => {
      expect(text).toContain(headline);
      expect(text).toContain(copy.hero.body);
    });

    test("shows every section heading in order", () => {
      const headings = [
        copy.hero.headline,
        copy.problem.title,
        copy.how.title,
        copy.features.title,
        copy.guardrails.title,
        copy.concurrency.title,
        copy.teams.title,
        copy.portable.title,
        copy.pricing.title,
        copy.faq.title,
        copy.closing.title,
      ];
      const rendered = [...html.matchAll(/<h[12][^>]*>(.*?)<\/h[12]>/gs)].map((match) =>
        textOf(match[1] ?? "").trim(),
      );
      expect(rendered).toEqual(headings);
    });

    test("shows the four plans with their prices and features", () => {
      for (const [name, price] of plans) {
        expect(text).toContain(name);
        expect(text).toContain(price);
      }
      for (const plan of copy.pricing.plans) {
        for (const item of plan.items) expect(text).toContain(item);
      }
      expect(text).toContain(copy.pricing.featured);
    });

    test("shows the seven FAQ questions with the first answer", () => {
      expect(copy.faq.items).toHaveLength(7);
      for (const item of copy.faq.items) expect(text).toContain(item.q);
      expect(html).toMatch(/<details[^>]*\sopen=""[^>]*>/);
      expect(countOf(html, ' open=""')).toBe(1);
      expect(text).toContain(copy.faq.items[0]?.a ?? "missing");
    });

    test("shows the real commands", () => {
      expect(text).toContain(
        "claude mcp add --transport http sdd-vault https://mcp-sddvault.eduindev.com/mcp",
      );
      expect(text).toContain("sddvault import acme/payments . --dry-run");
      expect(text).toContain("sddvault export acme/payments ./out");
      expect(text).not.toContain("Authorization: Bearer");
      expect(text).not.toContain("specstore");
    });

    test("links Start free, Sign in and Book a demo", () => {
      expect(countOf(html, `href="${app}/auth/login?action=register"`)).toBeGreaterThanOrEqual(5);
      expect(countOf(html, `href="${app}/auth/login"`)).toBeGreaterThanOrEqual(1);
      expect(
        countOf(html, 'href="mailto:hello@eduindev.com?subject=SDD%20Vault%20demo"'),
      ).toBeGreaterThanOrEqual(3);
    });

    test("has the anchors the header points to", () => {
      for (const id of ["how-it-works", "features", "security", "pricing", "faq"]) {
        expect(html).toContain(`href="#${id}"`);
        expect(html).toContain(`id="${id}"`);
      }
    });

    test("sets the footer link targets", () => {
      expect(html).toContain('href="mailto:security@eduindev.com"');
    });
  },
);
