import "@workspace/ui/globals.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { I18nextProvider } from "react-i18next";
import { createBrowserRouter } from "react-router";
import { RouterProvider } from "react-router/dom";
import { createI18n } from "~/lib/i18n";
import { routes } from "~/router";

const i18n = createI18n();
document.documentElement.lang = i18n.language;
const router = createBrowserRouter(routes);

const element = document.getElementById("root");
if (element) {
  createRoot(element).render(
    <StrictMode>
      <I18nextProvider i18n={i18n}>
        <RouterProvider router={router} />
      </I18nextProvider>
    </StrictMode>,
  );
}
