import { Button, Icon, message, StatusTag, Tag, Tooltip } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData, useNavigate } from "react-router";
import { api, unwrap } from "~/api/client";
import type { components } from "~/api/schema";
import { SpecProgress } from "~/components/spec-progress";
import { branch, type SpecKind, specNumber } from "~/lib/format";
import { useLiveRevalidate } from "~/lib/live";
import { guard } from "~/lib/session";
import { specTag } from "~/lib/spec-status";
import { projectParams } from "./project";

type ActiveSpec = components["schemas"]["ActiveSpecOutput"];
type DraftSpec = components["schemas"]["DraftSpecOutput"];

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  return guard(request, async () => {
    const [project, active] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      api.GET("/api/v1/projects/{org}/{project}/active", { params: { path } }).then(unwrap),
    ]);
    return { project, ...active };
  });
}

function NextTask({ spec, base }: { spec: ActiveSpec; base: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const task = spec.next_task;
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(spec.agent_prompt ?? "");
      message.success(t("active.copied"));
    } catch {
      message.error(t("active.copyFailed"));
    }
  };
  if (!task) {
    return (
      <p className="m-0 rounded bg-[var(--success-100)] px-4 py-3.5 text-sm text-[var(--success-700)]">
        {t("active.allDone")}
      </p>
    );
  }
  return (
    <section
      aria-label={t("active.nextTask")}
      className="flex flex-col gap-1.5 rounded bg-[var(--primary-50)] px-4 py-3.5 shadow-[inset_0_0_0_1px_var(--primary-300)]"
    >
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-xs font-medium uppercase text-[var(--primary-700)]">
          {t("active.nextTask")}
        </span>
        <span className="font-mono text-[13px] font-medium leading-5">{task.id}</span>
        {task.parallel && (
          <Tooltip title={t("spec.parallelTip")}>
            <span className="cursor-help rounded-sm bg-[var(--primary-100)] px-[5px] font-mono text-xs font-medium leading-[18px] text-[var(--primary-700)]">
              P
            </span>
          </Tooltip>
        )}
        {task.frs.map((fr) => (
          <span
            key={fr}
            className="rounded-sm px-[5px] font-mono text-xs leading-[18px] text-[var(--neutral-600)] shadow-[inset_0_0_0_1px_var(--neutral-300)]"
          >
            {fr}
          </span>
        ))}
      </div>
      <span className="text-sm leading-[22px]">{task.description}</span>
      {task.done_when && (
        <span className="text-[13px] leading-5 text-[var(--neutral-500)]">
          {t("active.doneWhen", { text: task.done_when })}
        </span>
      )}
      <div className="mt-1.5 flex flex-wrap items-center gap-2">
        <Tooltip title={t("active.copyTip")}>
          <Button
            size="small"
            leftIcon={<Icon name="Copy" size={14} />}
            onClick={() => void copy()}
          >
            {t("active.copyPrompt")}
          </Button>
        </Tooltip>
        <Button
          size="small"
          variant="ghost"
          onClick={() => navigate(`${base}/specs/${spec.number}/tasks`)}
        >
          {t("active.allTasks")}
        </Button>
      </div>
    </section>
  );
}

function ActiveCard({ spec, base }: { spec: ActiveSpec; base: string }) {
  const { t } = useTranslation();
  return (
    <section
      aria-label={t("active.activeLabel")}
      className="flex min-w-0 flex-col gap-4 rounded bg-white p-6 [flex:2_1_560px]"
    >
      <div className="flex flex-wrap items-center gap-2.5">
        <span className="text-xs uppercase text-[var(--neutral-500)]">
          {t("active.activeLabel")}
        </span>
        <span className="flex-1" />
        <span className="font-mono text-xs text-[var(--neutral-500)]">⑂ {spec.branch}</span>
      </div>
      <div className="flex flex-wrap items-center gap-2.5">
        <span className="font-mono text-xl font-medium leading-7 text-[var(--neutral-500)]">
          {specNumber(spec.number)}
        </span>
        <Link
          to={`${base}/specs/${spec.number}`}
          className="text-xl font-medium leading-7 text-inherit no-underline hover:underline"
        >
          {spec.title}
        </Link>
        <StatusTag status={specTag(spec.status)} size="medium">
          {t(`spec.statuses.${spec.status as "draft"}`)}
        </StatusTag>
      </div>
      <div className="grid gap-4 [grid-template-columns:repeat(auto-fit,minmax(140px,1fr))]">
        <div className="flex flex-col gap-0.5">
          <span className="text-xs text-[var(--neutral-500)]">{t("active.progress")}</span>
          <span className="font-numeric text-2xl font-medium leading-8">
            {t("active.tasksOf", { done: spec.tasks_done, total: spec.tasks_total })}
          </span>
        </div>
        <div className="flex flex-col gap-0.5">
          <span className="text-xs text-[var(--neutral-500)]">{t("active.phase")}</span>
          <span className="text-base font-medium leading-8">{spec.current_phase ?? "—"}</span>
        </div>
        <div className="flex flex-col gap-0.5">
          <span className="text-xs text-[var(--neutral-500)]">{t("active.markers")}</span>
          <span className="font-numeric text-2xl font-medium leading-8">{spec.open_markers}</span>
        </div>
      </div>
      <SpecProgress done={spec.tasks_done} total={spec.tasks_total} label={t("active.progress")} />
      <NextTask spec={spec} base={base} />
    </section>
  );
}

function DraftCard({ draft, base }: { draft: DraftSpec; base: string }) {
  const { t } = useTranslation();
  return (
    <section
      aria-label={`${specNumber(draft.number)} ${draft.title}`}
      className="flex flex-col gap-4 rounded bg-white p-6"
    >
      <div className="flex items-center gap-2.5">
        <span className="text-xs uppercase text-[var(--neutral-500)]">
          {t("active.draftLabel")}
        </span>
        <span className="flex-1" />
        <span className="font-mono text-xs text-[var(--neutral-500)]">
          ⑂ {branch(draft.number, draft.slug)}
        </span>
      </div>
      <div className="flex flex-wrap items-center gap-2.5">
        <span className="font-mono text-xl font-medium leading-7 text-[var(--neutral-500)]">
          {specNumber(draft.number)}
        </span>
        <Link
          to={`${base}/specs/${draft.number}`}
          className="text-xl font-medium leading-7 text-inherit no-underline hover:underline"
        >
          {draft.title}
        </Link>
        <Tag>{t("spec.statuses.draft")}</Tag>
      </div>
      {draft.summary && (
        <p className="m-0 text-sm leading-[22px] text-[var(--neutral-600)]">{draft.summary}</p>
      )}
      {draft.markers.length === 0 ? (
        <p className="m-0 text-[13px] text-[var(--neutral-500)]">{t("active.noQuestions")}</p>
      ) : (
        <div className="flex flex-col gap-2 rounded bg-[var(--warning-100)] px-4 py-3.5 shadow-[inset_0_0_0_1px_var(--warning-300)]">
          <h2 className="m-0 text-xs font-medium uppercase text-[var(--warning-700)]">
            {t("active.openQuestions")} · {draft.markers.length}
          </h2>
          <ul className="m-0 flex list-none flex-col gap-1.5 p-0">
            {draft.markers.map((marker) => (
              <li
                key={`${marker.document_kind}-${marker.line}`}
                className="flex flex-col text-sm leading-[22px]"
              >
                <span>{marker.text}</span>
                <span className="text-xs text-[var(--neutral-500)]">
                  {t(`spec.kinds.${marker.document_kind as SpecKind}`)} · {marker.heading}
                </span>
              </li>
            ))}
          </ul>
          <div>
            <Link
              to={`${base}/specs/${draft.number}/markers`}
              className="text-sm font-medium text-[var(--primary-600)] no-underline hover:underline"
            >
              {t("active.answer")}
            </Link>
          </div>
        </div>
      )}
    </section>
  );
}

export function ActiveSpecs() {
  const { project, active, drafts } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const navigate = useNavigate();
  useLiveRevalidate(project.address);
  const base = `/p/${project.address}`;
  return (
    <div className="mx-auto flex max-w-[1280px] flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3 pt-2">
        <h1 className="m-0 whitespace-nowrap text-2xl font-medium leading-8">
          {t("active.title")}
        </h1>
        <span className="ml-1 text-[13px] text-[var(--neutral-500)]">{t("active.subtitle")}</span>
        <span className="flex-1" />
        <Button variant="outline" onClick={() => navigate(`${base}/specs`)}>
          {t("active.allSpecs")}
        </Button>
      </div>
      {!active && drafts.length === 0 ? (
        <div className="flex flex-col items-center gap-2 rounded bg-white px-6 pb-10 pt-12 text-center">
          <span className="flex size-12 items-center justify-center rounded-full bg-[var(--primary-100)] text-[var(--primary-700)]">
            <Icon name="File" size={24} />
          </span>
          <h2 className="m-0 text-base font-medium leading-6">{t("active.emptyTitle")}</h2>
          <p className="m-0 text-[13px] text-[var(--neutral-500)]">{t("active.emptyBody")}</p>
          <Button variant="outline" size="small" onClick={() => navigate(`${base}/specs`)}>
            {t("active.openSpecs")}
          </Button>
        </div>
      ) : (
        <div className="flex flex-wrap items-stretch gap-4">
          {active && <ActiveCard spec={active} base={base} />}
          {drafts.length > 0 && (
            <div className="flex min-w-0 flex-col gap-4 [flex:1_1_360px]">
              {drafts.map((draft) => (
                <DraftCard key={draft.number} draft={draft} base={base} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
