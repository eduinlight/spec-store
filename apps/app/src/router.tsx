import type { RouteObject } from "react-router";
import * as audit from "~/routes/audit";
import * as constitution from "~/routes/constitution";
import * as diff from "~/routes/diff";
import * as history from "~/routes/history";
import * as home from "~/routes/home";
import * as invitationAccept from "~/routes/invitation-accept";
import * as invitations from "~/routes/invitations";
import * as members from "~/routes/members";
import * as onboarding from "~/routes/onboarding";
import * as organization from "~/routes/organization";
import * as overview from "~/routes/overview";
import * as placeholders from "~/routes/placeholder";
import * as profile from "~/routes/profile";
import * as projectSettings from "~/routes/project-settings";
import * as revision from "~/routes/revision";
import * as root from "~/routes/root";
import * as spec from "~/routes/spec";
import * as specDocument from "~/routes/spec-document";
import * as specHistory from "~/routes/spec-history";
import * as specMarkers from "~/routes/spec-markers";
import * as specTasks from "~/routes/spec-tasks";
import * as specTraceability from "~/routes/spec-traceability";
import * as specs from "~/routes/specs";

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
          { index: true, loader: home.loader, Component: home.Home },
          {
            path: "onboarding",
            loader: onboarding.loader,
            action: onboarding.action,
            Component: onboarding.Onboarding,
          },
          {
            path: "o/:org",
            loader: organization.loader,
            Component: organization.Organization,
            children: [{ path: "projects/new", Component: organization.NewProject }],
          },
          { path: "o/:org/audit", loader: audit.loader, Component: audit.Audit },
          {
            path: "o/:org/members",
            loader: members.loader,
            action: members.action,
            Component: members.Members,
          },
          {
            path: "o/:org/invitations",
            loader: invitations.loader,
            action: invitations.action,
            Component: invitations.Invitations,
          },
          {
            path: "invitations/:token",
            loader: invitationAccept.loader,
            action: invitationAccept.action,
            Component: invitationAccept.InvitationAccept,
          },
          {
            path: "profile",
            loader: profile.loader,
            action: profile.action,
            Component: profile.Profile,
          },
          { path: "p/:org/:project/active", Component: placeholders.ActiveSpecsPlaceholder },
          { path: "p/:org/:project/specs", loader: specs.loader, Component: specs.Specs },
          { path: "p/:org/:project/activity", Component: placeholders.ActivityPlaceholder },
          { path: "p/:org/:project/members", Component: placeholders.MembersPlaceholder },
          { path: "account/tokens", Component: placeholders.TokensPlaceholder },
          { path: "account/preferences", Component: placeholders.PreferencesPlaceholder },
          { path: "o/:org/security", Component: placeholders.SecurityPlaceholder },
          { path: "o/:org/billing", Component: placeholders.BillingPlaceholder },
          {
            path: "p/:org/:project/settings",
            loader: projectSettings.loader,
            action: projectSettings.action,
            Component: projectSettings.ProjectSettings,
          },
          { path: "p/:org/:project", loader: overview.loader, Component: overview.Overview },
          {
            path: "p/:org/:project/constitution",
            loader: constitution.loader,
            Component: constitution.Constitution,
          },
          {
            id: "spec",
            path: "p/:org/:project/specs/:number",
            loader: spec.loader,
            shouldRevalidate: spec.shouldRevalidate,
            Component: spec.Spec,
            children: [
              { index: true, loader: specDocument.loader, Component: specDocument.SpecDocument },
              { path: "tasks", loader: specTasks.loader, Component: specTasks.SpecTasks },
              {
                path: "traceability",
                loader: specTraceability.loader,
                Component: specTraceability.SpecTraceability,
              },
              { path: "markers", loader: specMarkers.loader, Component: specMarkers.SpecMarkers },
              { path: "history", loader: specHistory.loader, Component: specHistory.SpecHistory },
              { path: ":kind", loader: specDocument.loader, Component: specDocument.SpecDocument },
            ],
          },
          { path: "p/:org/:project/history", loader: history.loader, Component: history.History },
          {
            path: "p/:org/:project/revisions/:id",
            loader: revision.loader,
            Component: revision.Revision,
          },
          { path: "p/:org/:project/revisions/:id/diff", loader: diff.loader, Component: diff.Diff },
          { path: "p/:org/:project/audit", loader: audit.loader, Component: audit.Audit },
        ],
      },
    ],
  },
];
