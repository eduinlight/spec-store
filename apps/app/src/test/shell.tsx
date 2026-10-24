import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { I18nextProvider } from "react-i18next";
import { MemoryRouter, useLocation } from "react-router";
import { createI18n } from "~/lib/i18n";
import type { Language } from "~/lib/language";

export function LocationProbe() {
  const location = useLocation();
  return <output data-testid="location">{location.pathname + location.search}</output>;
}

export function renderShell(ui: ReactNode, path = "/", language: Language = "en") {
  const i18n = createI18n(language);
  return {
    i18n,
    ...render(
      <I18nextProvider i18n={i18n}>
        <MemoryRouter initialEntries={[path]}>
          {ui}
          <LocationProbe />
        </MemoryRouter>
      </I18nextProvider>,
    ),
  };
}
