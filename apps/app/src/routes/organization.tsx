import { Card } from "@workspace/ui/components/card";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { guard } from "~/lib/session";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const org = params.org ?? "";
  return guard(request, async () => {
    const [organization, projects] = await Promise.all([
      api.GET("/api/v1/orgs/{org}", { params: { path: { org } } }).then(unwrap),
      api.GET("/api/v1/orgs/{org}/projects", { params: { path: { org } } }).then(unwrap),
    ]);
    return { organization, projects: projects.projects };
  });
}

export function Organization() {
  const { organization, projects } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const owner = organization.role === "owner";
  return (
    <div className="grid gap-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">{organization.name}</h1>
          <p className="text-sm text-muted-foreground">{organization.slug}</p>
        </div>
        <nav className="flex gap-4 text-sm">
          <Link to={`/o/${organization.slug}/members`} className="hover:underline">
            {t("organization.members")}
          </Link>
          {owner && (
            <>
              <Link to={`/o/${organization.slug}/invitations`} className="hover:underline">
                {t("organization.invitations")}
              </Link>
              <Link to={`/o/${organization.slug}/audit`} className="hover:underline">
                {t("organization.audit")}
              </Link>
              <Link to={`/o/${organization.slug}/projects/new`} className="hover:underline">
                {t("organization.newProject")}
              </Link>
            </>
          )}
        </nav>
      </div>
      <Card>
        <h2 className="text-lg font-semibold">{t("organization.projects")}</h2>
        {projects.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("organization.noProjects")}</p>
        ) : (
          <ul className="divide-y">
            {projects.map((project) => (
              <li key={project.address} className="flex items-center justify-between py-3">
                <Link
                  to={`/p/${project.address}`}
                  className="font-medium text-primary hover:underline"
                >
                  {project.name}
                </Link>
                <span className="text-sm text-muted-foreground">
                  {project.role ?? t("app.operator")}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}
