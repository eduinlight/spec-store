import { Avatar, LockDocument, StatusTag, Tag, Tooltip } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import type { components } from "~/api/schema";
import { useDate } from "~/lib/format";
import { specTag } from "~/lib/spec-status";

export type ProjectSummary = components["schemas"]["ProjectSummaryOutput"];

const percent = (done: number, total: number) => (total > 0 ? Math.round((done / total) * 100) : 0);

export function ProjectCard({ project }: { project: ProjectSummary }) {
  const { t } = useTranslation();
  const date = useDate();
  const spec = project.active_spec;
  const activity = project.last_activity;
  const progress = spec ? percent(spec.tasks_done, spec.tasks_total) : 0;
  return (
    <Link
      to={`/p/${project.address}`}
      className="flex h-full flex-col gap-3.5 rounded bg-white px-6 py-5 text-inherit no-underline transition-shadow hover:shadow-[var(--elevation-2)] focus-visible:shadow-[var(--focus-ring)] focus-visible:outline-none"
    >
      <span className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate text-lg font-medium leading-[26px] text-[var(--neutral-700)]">
          {project.name}
        </span>
        {project.archived && <Tag>{t("card.archived")}</Tag>}
        <Tag type={project.role === "owner" ? "primary" : "basic"}>
          {project.role ? t(`projects.roles.${project.role as "owner"}`) : t("app.operator")}
        </Tag>
      </span>
      <span className="flex items-center gap-2 text-xs text-[var(--neutral-500)]">
        <LockDocument size={14} />
        <span>
          {project.constitution_version
            ? t("card.constitution", { version: project.constitution_version })
            : t("card.noConstitution")}
          {project.constitution_version && project.constitution_updated_at
            ? ` · ${t("card.constitutionUpdated", { when: date(project.constitution_updated_at) })}`
            : ""}
        </span>
      </span>
      <span className="flex flex-col gap-2.5 rounded bg-[var(--neutral-50)] p-3">
        {spec ? (
          <>
            <span className="flex items-center gap-2">
              <span className="font-mono text-[13px] font-medium text-[var(--neutral-500)]">
                {String(spec.number).padStart(3, "0")}
              </span>
              <span className="min-w-0 flex-1 truncate font-mono text-sm font-medium text-[var(--neutral-700)]">
                {spec.slug}
              </span>
              <StatusTag status={specTag(spec.status)}>
                {t(`spec.statuses.${spec.status as "draft"}`)}
              </StatusTag>
            </span>
            <span className="flex items-center gap-2.5">
              <span
                role="progressbar"
                aria-label={t("card.progressLabel")}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={progress}
                className="h-1.5 flex-1 overflow-hidden rounded-full bg-[var(--neutral-200)]"
              >
                <span
                  className="block h-full bg-[var(--primary-600)]"
                  style={{ width: `${progress}%` }}
                />
              </span>
              <span className="text-xs text-[var(--neutral-500)]">
                {t("card.progress", { done: spec.tasks_done, total: spec.tasks_total })}
              </span>
            </span>
          </>
        ) : (
          <span className="text-sm text-[var(--neutral-500)]">{t("card.noSpecs")}</span>
        )}
      </span>
      <span className="mt-auto flex items-center gap-2 text-xs leading-[18px] text-[var(--neutral-500)]">
        {activity ? (
          <>
            <Avatar name={activity.actor} size={20} />
            <span className="min-w-0 flex-1 truncate">
              {activity.actor}
              {activity.client && (
                <>
                  {" · "}
                  <span className="rounded-sm bg-[var(--neutral-100)] px-1 text-[var(--neutral-600)]">
                    {activity.client}
                  </span>
                </>
              )}
              {` · ${date(activity.at)}`}
            </span>
          </>
        ) : (
          <span className="flex-1">{t("card.noActivity")}</span>
        )}
        {project.open_markers > 0 && (
          <Tooltip title={t("card.markersTip", { count: project.open_markers })}>
            <span className="cursor-help whitespace-nowrap text-[var(--warning-700)]">
              {t("card.markers", { count: project.open_markers })}
            </span>
          </Tooltip>
        )}
      </span>
      {project.archived && (
        <span className="text-xs text-[var(--neutral-500)]">{t("card.archivedNote")}</span>
      )}
    </Link>
  );
}
