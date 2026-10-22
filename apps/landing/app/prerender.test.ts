import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const client = resolve(import.meta.dirname, "../build/client");
const english = readFileSync(resolve(client, "index.html"), "utf8");
const spanish = readFileSync(resolve(client, "es/index.html"), "utf8");
const app = process.env.VITE_APP_URL ?? "https://app-spec-store.eduindev.com";

test("the English page is complete in the first HTML response", () => {
  expect(english).toContain('<html lang="en"');
  expect(english).toContain("Specs your agents can trust");
  expect(english).toContain("one place your team and your AI agents share");
  expect(english).toContain(`href="${app}/auth/login?action=register"`);
  expect(english).toContain(`href="${app}/auth/login"`);
  expect(english).toContain('hreflang="es"');
});

test("the Spanish page is complete in the first HTML response", () => {
  expect(spanish).toContain('<html lang="es"');
  expect(spanish).toContain("Specs en las que tus agentes pueden confiar");
  expect(spanish).toContain(`href="${app}/auth/login?action=register"`);
  expect(spanish).toContain('hreflang="en"');
});
