import { Avatar, CheckableTag, Tooltip } from "@workspace/ui/taptap";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { type LoaderFunctionArgs, useLoaderData, useRouteLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { optional } from "~/api/optional";
import { commitUrl } from "~/lib/commit";
import { useDate } from "~/lib/format";
import { guard } from "~/lib/session";
import {
  applyFilters,
  groupByPhase,
  TASK_FILTERS,
  type TaskFilter,
  toggleFilter,
} from "~/lib/spec-tasks";
import { projectParams } from "./project";
import type { RootData } from "./root";
import type { SpecLayoutData } from "./spec";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const specPath = { ...path, number: Number(params.number) };
  return guard(request, async () => {
    const [tasks, traceability] = await Promise.all([
      optional(
        api
          .GET("/api/v1/projects/{org}/{project}/specs/{number}/tasks", {
            params: { path: specPath },
          })
          .then(unwrap),
      ),
      optional(
        api
          .GET("/api/v1/projects/{org}/{project}/specs/{number}/traceability", {
            params: { path: specPath },
          })
          .then(unwrap),
      ),
    ]);
    return {
      tasks: tasks?.tasks ?? null,
      requirements: new Map(
        (traceability?.requirements ?? []).map((requirement) => [requirement.id, requirement.text]),
      ),
    };
  });
}

export function SpecTasks() {
  const { tasks, requirements } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const layout = useRouteLoaderData("spec") as SpecLayoutData;
  const root = useRouteLoaderData("root") as RootData;
  const { t } = useTranslation();
  const date = useDate();
  const [active, setActive] = useState<TaskFilter[]>([]);

  if (tasks === null) {
    return (
      <p className="m-0 rounded bg-white px-6 py-12 text-center text-sm text-[var(--neutral-500)]">
        {t("spec.notWritten", { document: t("spec.kinds.tasks") })}
      </p>
    );
  }

  const shown = applyFilters(tasks, active, root.me.handle);
  const phases = groupByPhase(shown);
  const doneTotal = tasks.filter((task) => task.done).length;

  return (
    <div className="flex flex-col gap-4 rounded bg-white px-6 pb-6 pt-4">
      <div className="flex flex-wrap items-center gap-2">
        <fieldset className="m-0 flex flex-wrap gap-2 border-0 p-0">
          <legend className="sr-only">{t("spec.taskFiltersLabel")}</legend>
          {TASK_FILTERS.map((filter) => (
            <CheckableTag
              key={filter}
              size="small"
              checked={active.includes(filter)}
              onChange={() => setActive(toggleFilter(active, filter))}
            >
              {t(`spec.taskFilters.${filter}`)}
            </CheckableTag>
          ))}
        </fieldset>
        <span className="flex-1" />
        <span className="text-[13px] leading-5 text-[var(--neutral-500)]">
          {t("spec.taskSummary", { done: doneTotal, total: tasks.length })}
        </span>
      </div>
      {phases.map((phase) => (
        <section key={phase.name} aria-label={phase.name || t("spec.noPhase")}>
          <div className="flex items-center gap-2.5 border-b border-[var(--neutral-200)] py-2.5">
            <h2 className="m-0 text-sm font-medium leading-[22px]">
              {phase.name || t("spec.noPhase")}
            </h2>
            <span className="text-xs text-[var(--neutral-500)]">
              {phase.tasks.filter((task) => task.done).length}/{phase.tasks.length}
            </span>
          </div>
          <ul className="m-0 list-none p-0">
            {phase.tasks.map((task) => {
              const url = task.commit
                ? commitUrl(layout.project.repository_url, task.commit)
                : null;
              return (
                <li
                  key={task.id}
                  className="grid grid-cols-[24px_44px_minmax(0,1fr)_auto] items-start gap-3 border-b border-[var(--neutral-200)] py-3"
                >
                  <span
                    role="img"
                    aria-label={task.done ? t("spec.taskDone") : t("spec.taskPending")}
                    className={
                      task.done
                        ? "mt-[3px] flex size-4 items-center justify-center rounded-sm bg-[var(--primary-600)] text-[11px] text-white"
                        : "mt-[3px] flex size-4 items-center justify-center rounded-sm text-[11px] text-white shadow-[inset_0_0_0_1px_var(--neutral-300)]"
                    }
                  >
                    {task.done ? "✓" : ""}
                  </span>
                  <span className="font-mono text-[13px] font-medium leading-[22px]">
                    {task.id}
                  </span>
                  <div className="flex min-w-0 flex-col gap-1">
                    <div className="flex flex-wrap items-center gap-1.5">
                      <span
                        className={
                          task.done
                            ? "text-sm leading-[22px] text-[var(--neutral-500)]"
                            : "text-sm leading-[22px] text-[var(--neutral-700)]"
                        }
                      >
                        {task.description}
                      </span>
                      {task.parallel && (
                        <Tooltip title={t("spec.parallelTip")}>
                          <span className="cursor-help rounded-sm bg-[var(--primary-100)] px-[5px] font-mono text-xs font-medium leading-[18px] text-[var(--primary-700)]">
                            P
                          </span>
                        </Tooltip>
                      )}
                      {task.frs.map((fr) => (
                        <Tooltip key={fr} title={requirements.get(fr) ?? ""}>
                          <span className="cursor-help rounded-sm px-[5px] font-mono text-xs leading-[18px] text-[var(--neutral-600)] shadow-[inset_0_0_0_1px_var(--neutral-300)]">
                            {fr}
                          </span>
                        </Tooltip>
                      ))}
                    </div>
                    {task.done_when && (
                      <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                        {t("spec.doneWhenLine", { text: task.done_when })}
                      </span>
                    )}
                  </div>
                  <div className="flex items-center justify-end gap-2 whitespace-nowrap text-xs text-[var(--neutral-500)]">
                    {task.done && task.completed_by && (
                      <Tooltip
                        title={t("spec.completedTip", {
                          author: task.completed_by.author,
                          when: date(task.completed_by.at),
                          revision: task.completed_by.revision,
                        })}
                      >
                        <span className="flex items-center gap-1.5">
                          <Avatar name={task.completed_by.author} size={20} />
                          <span>{task.completed_by.client ?? task.completed_by.author}</span>
                        </span>
                      </Tooltip>
                    )}
                    {task.done && task.commit && <CommitChip commit={task.commit} url={url} />}
                  </div>
                </li>
              );
            })}
          </ul>
        </section>
      ))}
      {shown.length === 0 && (
        <p className="m-0 py-12 text-center text-sm text-[var(--neutral-500)]">
          {t("spec.noTasksMatch")}
        </p>
      )}
    </div>
  );
}

export function CommitChip({ commit, url }: { commit: string; url: string | null }) {
  const className =
    "rounded-sm bg-[var(--neutral-100)] px-1.5 py-px font-mono text-[var(--neutral-700)] no-underline";
  return url ? (
    <a href={url} className={className} rel="noreferrer">
      {commit}
    </a>
  ) : (
    <span className={className}>{commit}</span>
  );
}
