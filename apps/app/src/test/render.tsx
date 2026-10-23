import { render } from "@testing-library/react";
import { I18nextProvider } from "react-i18next";
import { createMemoryRouter, RouterProvider } from "react-router";
import { createI18n } from "~/lib/i18n";
import type { Language } from "~/lib/language";
import { routes } from "~/router";

export function renderAt(path: string, language: Language = "en") {
  const i18n = createI18n(language);
  const router = createMemoryRouter(routes, { initialEntries: [path] });
  const view = render(
    <I18nextProvider i18n={i18n}>
      <RouterProvider router={router} />
    </I18nextProvider>,
  );
  return { ...view, router, i18n };
}
