import type { RouteObject } from "react-router";
import * as document from "~/routes/document";
import * as home from "~/routes/home";
import * as onboarding from "~/routes/onboarding";
import * as organization from "~/routes/organization";
import * as project from "~/routes/project";
import * as root from "~/routes/root";
import * as spec from "~/routes/spec";

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
          { path: "p/:org/:project", loader: project.loader, Component: project.Project },
          {
            path: "p/:org/:project/constitution",
            loader: document.loader,
            Component: document.Document,
          },
          { path: "p/:org/:project/specs/:number", loader: spec.loader, Component: spec.Spec },
          {
            path: "p/:org/:project/specs/:number/:kind",
            loader: document.loader,
            Component: document.Document,
          },
        ],
      },
    ],
  },
];
