import { Card } from "@workspace/ui/components/card";
import { Alert, Tag } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData, useRouteLoaderData } from "react-router";
import { ApiFailure, api, unwrap } from "~/api/client";
import { GetStarted } from "~/components/get-started";
import { connectCommand, isAgentClient, mcpUrlOf } from "~/lib/mcp";
import { guard } from "~/lib/session";
import type { RootData } from "~/routes/root";

export function projectParams(params: Record<string, string | undefined>) {
  return { org: params.org ?? "", project: params.project ?? "" };
}

async function summaryOf(org: string, address: string) {
  try {
    const listed = await api
      .GET("/api/v1/orgs/{org}/projects", { params: { path: { org } } })
      .then(unwrap);
    return listed.projects.find((item) => item.address === address) ?? null;
  } catch (error) {
    if (error instanceof ApiFailure) return null;
    throw error;
  }
}

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  return guard(request, async () => {
    const [project, status, members, summary] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      api.GET("/api/v1/projects/{org}/{project}/status", { params: { path } }).then(unwrap),
      api.GET("/api/v1/projects/{org}/{project}/members", { params: { path } }).then(unwrap),
      summaryOf(path.org, `${path.org}/${path.project}`),
    ]);
    return { project, status, members: members.members, summary };
  });
}

export function Project() {
  const { project, status, members, summary } = useLoaderData() as Awaited<
    ReturnType<typeof loader>
  >;
  const root = useRouteLoaderData("root") as RootData | undefined;
  const { t } = useTranslation();
  const base = `/p/${project.address}`;
  const owner = project.role === "owner";
  const specWritten = status.specs.some((spec) => spec.number === 1);
  const agentCalled = isAgentClient(summary?.last_activity?.client);
  const gettingStarted = owner && !project.archived && !specWritten;
  return (
    <div className="grid gap-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-sm text-muted-foreground">
            <Link to={`/o/${project.organization}`} className="hover:underline">
              {project.organization}
            </Link>
          </p>
          <h1 className="flex items-center gap-3 text-2xl font-semibold">
            {project.name}
            {project.archived && <Tag>{t("card.archived")}</Tag>}
          </h1>
          <p className="text-sm text-muted-foreground">{project.address}</p>
        </div>
        {owner && (
          <nav className="flex gap-4 text-sm">
            <Link to={`${base}/settings`} className="hover:underline">
              {t("projects.settings")}
            </Link>
            <Link to={`${base}/audit`} className="hover:underline">
              {t("organization.audit")}
            </Link>
          </nav>
        )}
      </div>
      {project.archived && (
        <Alert status="info" closable={false}>
          {t("archive.banner")}
        </Alert>
      )}
      {gettingStarted && (
        <GetStarted
          command={connectCommand(mcpUrlOf(root?.me))}
          agentCalled={agentCalled}
          specWritten={specWritten}
        />
      )}
      <div className="grid gap-6 md:grid-cols-3">
        <Card className="md:col-span-2">
          <h2 className="text-lg font-semibold">{t("project.specs")}</h2>
          {status.specs.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("project.noSpecs")}</p>
          ) : (
            <ul className="divide-y">
              {status.specs.map((spec) => (
                <li
                  key={spec.number}
                  className="flex flex-wrap items-center justify-between gap-2 py-3"
                >
                  <Link
                    to={`${base}/specs/${spec.number}`}
                    className="font-medium text-primary hover:underline"
                  >
                    {spec.branch}
                  </Link>
                  <span className="text-sm text-muted-foreground">
                    {t(`spec.statuses.${spec.status as "draft"}`)} ·{" "}
                    {spec.kinds.map((kind) => t(`spec.kinds.${kind as "spec"}`)).join(", ")}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Card>
        <div className="grid content-start gap-6">
          <Card>
            <h2 className="text-lg font-semibold">{t("project.constitution")}</h2>
            {status.constitution_version ? (
              <Link to={`${base}/constitution`} className="text-primary hover:underline">
                {t("project.version", { version: status.constitution_version })}
              </Link>
            ) : (
              <p className="text-sm text-muted-foreground">{t("project.noConstitution")}</p>
            )}
            {project.repository_url && (
              <div className="text-sm">
                <span className="text-muted-foreground">{t("project.repository")}: </span>
                <a
                  href={project.repository_url}
                  className="text-primary hover:underline"
                  rel="noreferrer"
                >
                  {project.repository_url}
                </a>
              </div>
            )}
          </Card>
          {status.active && (
            <Card>
              <h2 className="text-lg font-semibold">{t("project.activeSpec")}</h2>
              <Link
                to={`${base}/specs/${status.active.number}`}
                className="text-primary hover:underline"
              >
                {status.active.branch}
              </Link>
              <p className="text-sm">
                {t("project.progress", {
                  done: status.active.tasks_done,
                  total: status.active.tasks_total,
                })}
              </p>
              {status.active.next_task && (
                <p className="text-sm text-muted-foreground">
                  {t("project.nextTask")}: {status.active.next_task.id}{" "}
                  {status.active.next_task.description}
                </p>
              )}
              <p className="text-sm text-muted-foreground">
                {t("project.markers")}: {status.active.markers}
              </p>
            </Card>
          )}
          <Card>
            <h2 className="text-lg font-semibold">{t("project.members")}</h2>
            <ul aria-label={t("project.members")} className="grid gap-1 text-sm">
              {members.map((member) => (
                <li key={member.handle} className="flex justify-between">
                  <span>{member.handle}</span>
                  <span className="text-muted-foreground">
                    {t(`projects.roles.${member.role as "owner"}`)}
                  </span>
                </li>
              ))}
            </ul>
          </Card>
        </div>
      </div>
    </div>
  );
}
