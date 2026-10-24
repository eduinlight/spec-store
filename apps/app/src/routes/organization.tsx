import { Button, Tabs, type TabsKey } from "@workspace/ui/taptap";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { type LoaderFunctionArgs, useLoaderData, useNavigate } from "react-router";
import { api, unwrap } from "~/api/client";
import { ProjectCard, type ProjectSummary } from "~/components/project-card";
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

export type ProjectFilter = "mine" | "all" | "archived";

const FILTERS: ProjectFilter[] = ["mine", "all", "archived"];

const FILTER_RULES: Record<ProjectFilter, (project: ProjectSummary) => boolean> = {
  mine: (project) => !project.archived && Boolean(project.role),
  all: (project) => !project.archived,
  archived: (project) => project.archived,
};

export function Organization() {
  const { organization, projects } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [filter, setFilter] = useState<ProjectFilter>("mine");
  const owner = organization.role === "owner";
  const visible = projects.filter(FILTER_RULES[filter]);
  const base = `/o/${organization.slug}/projects/new`;
  return (
    <div className="mx-auto flex max-w-[1280px] flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3 pt-2">
        <h1 className="m-0 flex-1 text-2xl font-medium leading-8">{t("home.title")}</h1>
        <Tabs
          type="card"
          items={FILTERS.map((value) => ({ value, label: t(`home.filters.${value}`) }))}
          value={filter}
          onChange={(value: TabsKey) => setFilter(value as ProjectFilter)}
        />
        {owner && (
          <>
            <Button variant="outline" onClick={() => navigate(`${base}?start=import`)}>
              {t("home.importTree")}
            </Button>
            <Button onClick={() => navigate(base)}>{t("home.createProject")}</Button>
          </>
        )}
      </div>
      {visible.length > 0 ? (
        <ul
          aria-label={t("home.list")}
          className="m-0 grid list-none gap-4 p-0 [grid-template-columns:repeat(auto-fill,minmax(min(100%,340px),1fr))]"
        >
          {visible.map((project) => (
            <li key={project.address}>
              <ProjectCard project={project} />
            </li>
          ))}
        </ul>
      ) : (
        <div className="flex flex-col items-center gap-2 rounded bg-white px-6 py-16 text-center">
          <span className="text-base font-medium leading-6">{t(`home.empty.${filter}.title`)}</span>
          <span className="text-[13px] text-[var(--neutral-500)]">
            {t(`home.empty.${filter}.body`)}
          </span>
        </div>
      )}
    </div>
  );
}
