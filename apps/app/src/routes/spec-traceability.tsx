import { Icon, Table, type TableColumn, Tooltip } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import { type LoaderFunctionArgs, useLoaderData, useRouteLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { commitUrl } from "~/lib/commit";
import { guard } from "~/lib/session";
import {
  commitsOf,
  coveragePercent,
  doneCount,
  requirementsWithTasks,
  type TraceRequirement,
  type TraceStatus,
  traceStatusOf,
} from "~/lib/spec-trace";
import { projectParams } from "./project";
import type { SpecLayoutData } from "./spec";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const specPath = { ...path, number: Number(params.number) };
  return guard(request, async () =>
    api
      .GET("/api/v1/projects/{org}/{project}/specs/{number}/traceability", {
        params: { path: specPath },
      })
      .then(unwrap),
  );
}

const STATUS_TONES: Record<TraceStatus, { background: string; color: string }> = {
  covered: { background: "var(--success-100)", color: "var(--success-700)" },
  in_progress: { background: "var(--info-100)", color: "var(--info-700)" },
  not_started: { background: "var(--neutral-100)", color: "var(--neutral-600)" },
  no_tasks: { background: "var(--warning-100)", color: "var(--warning-700)" },
};

interface Row {
  id: string;
  requirement: TraceRequirement;
}

export function SpecTraceability() {
  const { requirements, tasks_without_requirements: loose } = useLoaderData() as Awaited<
    ReturnType<typeof loader>
  >;
  const layout = useRouteLoaderData("spec") as SpecLayoutData;
  const { t } = useTranslation();

  const columns: TableColumn<Row>[] = [
    {
      key: "id",
      title: t("spec.trace.columns.fr"),
      width: 80,
      render: (_, row) => (
        <span className="font-mono text-[13px] font-medium text-[var(--primary-700)]">
          {row.requirement.id}
        </span>
      ),
    },
    {
      key: "text",
      title: t("spec.trace.columns.text"),
      maxWidth: 360,
      render: (_, row) => (
        <Tooltip title={row.requirement.text} placement="right">
          <span className="cursor-help">{row.requirement.text}</span>
        </Tooltip>
      ),
    },
    {
      key: "tasks",
      title: t("spec.trace.columns.tasks"),
      render: (_, row) => (
        <span className="flex flex-wrap gap-1">
          {row.requirement.tasks.map((task) => (
            <Tooltip
              key={task.id}
              title={t(task.done ? "spec.trace.taskDone" : "spec.trace.taskPending", {
                id: task.id,
                description: task.description,
              })}
            >
              <span
                className={
                  task.done
                    ? "rounded-sm bg-[var(--primary-100)] px-1.5 font-mono text-xs font-medium leading-5 text-[var(--primary-700)]"
                    : "rounded-sm px-1.5 font-mono text-xs font-medium leading-5 text-[var(--neutral-600)] shadow-[inset_0_0_0_1px_var(--neutral-300)]"
                }
              >
                {task.id}
              </span>
            </Tooltip>
          ))}
        </span>
      ),
    },
    {
      key: "done",
      title: t("spec.trace.columns.done"),
      width: 150,
      render: (_, row) => {
        const percent = coveragePercent(row.requirement);
        return (
          <span className="flex items-center gap-2">
            <span
              role="progressbar"
              aria-label={`${row.requirement.id} ${t("spec.trace.columns.done")}`}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={percent}
              className="h-1 flex-1 overflow-hidden rounded-full bg-[var(--neutral-200)]"
            >
              <span
                className="block h-full bg-[var(--primary-600)]"
                style={{ width: `${percent}%` }}
              />
            </span>
            <span className="text-xs text-[var(--neutral-500)]">
              {doneCount(row.requirement)}/{row.requirement.tasks.length}
            </span>
          </span>
        );
      },
    },
    {
      key: "commits",
      title: t("spec.trace.columns.commits"),
      width: 170,
      render: (_, row) => (
        <span className="flex flex-wrap gap-x-2 font-mono text-xs text-[var(--neutral-600)]">
          {commitsOf(row.requirement).map((commit) => {
            const url = commitUrl(layout.project.repository_url, commit);
            return url ? (
              <a key={commit} href={url} rel="noreferrer" className="text-inherit">
                {commit}
              </a>
            ) : (
              <span key={commit}>{commit}</span>
            );
          })}
        </span>
      ),
    },
    {
      key: "status",
      title: t("spec.trace.columns.status"),
      width: 130,
      render: (_, row) => {
        const status = traceStatusOf(row.requirement.status);
        const tone = STATUS_TONES[status];
        return (
          <span
            className="inline-flex h-[22px] items-center gap-1.5 rounded-full px-2 text-xs"
            style={tone}
          >
            <span
              aria-hidden="true"
              className="size-1.5 rounded-full"
              style={{ background: tone.color }}
            />
            {t(`spec.trace.statuses.${status}`)}
          </span>
        );
      },
    },
  ];

  const rows: Row[] = requirements.map((requirement) => ({ id: requirement.id, requirement }));

  return (
    <div className="flex flex-col gap-4 rounded bg-white px-6 pb-6 pt-5">
      <div className="flex flex-wrap items-center gap-3">
        <h2 className="m-0 text-lg font-medium leading-[26px]">{t("spec.trace.title")}</h2>
        <span className="flex-1 text-[13px] text-[var(--neutral-500)]">
          {t("spec.trace.summary", {
            covered: requirementsWithTasks(requirements),
            total: requirements.length,
          })}
        </span>
        <Tooltip title={t("spec.trace.help")} placement="left">
          <span className="flex cursor-help text-[var(--neutral-500)]">
            <Icon name="Help" size={16} title={t("spec.trace.help")} />
          </span>
        </Tooltip>
      </div>
      <Table<Row>
        caption={t("spec.trace.title")}
        columns={columns}
        data={rows}
        emptyText={t("spec.trace.none")}
      />
      {loose.length > 0 && (
        <div className="flex items-center gap-2 rounded bg-[var(--neutral-50)] px-3 py-2.5 text-[13px] leading-5 text-[var(--neutral-600)]">
          <span className="flex text-[var(--info-600)]">
            <Icon name="Info" size={16} />
          </span>
          <span>
            {t("spec.trace.withoutRequirements", {
              tasks: loose.map((task) => task.id).join(", "),
            })}
          </span>
        </div>
      )}
    </div>
  );
}
