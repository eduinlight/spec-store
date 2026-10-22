import type { RouteObject } from "react-router";
import * as home from "~/routes/home";
import * as onboarding from "~/routes/onboarding";
import * as organization from "~/routes/organization";
import * as root from "~/routes/root";

export const routes: RouteObject[] = [
  {
    id: "root",
    path: "/",
    loader: root.loader,
    Component: root.Root,
    ErrorBoundary: root.RouteError,
    children: [
      {
        errorElement: <root.RouteError />,
        children: [
          { index: true, loader: home.loader },
          {
            path: "onboarding",
            loader: onboarding.loader,
            action: onboarding.action,
            Component: onboarding.Onboarding,
          },
          { path: "o/:org", loader: organization.loader, Component: organization.Organization },
        ],
      },
    ],
  },
];
