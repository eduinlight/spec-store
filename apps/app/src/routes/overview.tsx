import { cn } from "@workspace/ui/lib/utils";
import {
  Alert,
  BarChart,
  Button,
  CheckableTag,
  Icon,
  type IconName,
  StatusTag,
  Tag,
} from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import {
  Link,
  type LoaderFunctionArgs,
  useLoaderData,
  useNavigate,
  useRouteLoaderData,
  useSearchParams,
} from "react-router";
import { ApiFailure, api, unwrap } from "~/api/client";
import { ActivityItem } from "~/components/activity-item";
import { GetStarted } from "~/components/get-started";
import { InfoTip } from "~/components/info-tip";
import { SpecProgress } from "~/components/spec-progress";
import { OVERVIEW_ACTIVITY_ROWS } from "~/lib/activity";
import { formatShortDay } from "~/lib/dates";
import { branch } from "~/lib/format";
import { useLiveRevalidate } from "~/lib/live";
import { connectCommand, isAgentClient, mcpUrlOf } from "~/lib/mcp";
import {
  attentionPath,
  chartData,
  coveragePercent,
  isAttentionKind,
  type OverviewData,
  PERIODS,
  periodOf,
} from "~/lib/overview";
import { guard } from "~/lib/session";
import { specTag } from "~/lib/spec-status";
import { projectParams } from "./project";
import type { RootData } from "./root";

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
  const days = periodOf(new URL(request.url).searchParams.get("days"));
  return guard(request, async () => {
    const [project, status, summary, overview, active, activity] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      api.GET("/api/v1/projects/{org}/{project}/status", { params: { path } }).then(unwrap),
      summaryOf(path.org, `${path.org}/${path.project}`),
      api
        .GET("/api/v1/projects/{org}/{project}/overview", {
          params: { path, query: { days } },
        })
        .then(unwrap),
      api.GET("/api/v1/projects/{org}/{project}/active", { params: { path } }).then(unwrap),
      api
        .GET("/api/v1/projects/{org}/{project}/activity", {
          params: { path, query: { limit: OVERVIEW_ACTIVITY_ROWS } },
        })
        .then(unwrap),
    ]);
    return { project, status, summary, overview, active, activity, days };
  });
}

interface KpiProps {
  label: string;
  tip: string;
  value: string;
  sub: string;
  tone?: "up" | "down" | "flat";
}

function Kpi({ label, tip, value, sub, tone = "flat" }: KpiProps) {
  return (
    <div className="flex flex-col gap-1 rounded bg-white px-6 py-5">
      <span className="flex items-center gap-1.5 text-xs text-[var(--neutral-500)]">
        {label}
        <InfoTip label={label} text={tip} />
      </span>
      <span className="font-numeric text-3xl font-medium leading-[38px] text-[var(--neutral-700)]">
        {value}
      </span>
      <span
        className={cn(
          "text-xs leading-[18px]",
          tone === "up" && "text-[var(--success-700)]",
          tone === "down" && "text-[var(--danger-700)]",
          tone === "flat" && "text-[var(--neutral-500)]",
        )}
      >
        {sub}
      </span>
    </div>
  );
}

const ATTENTION_KEYS = {
  approval_waiting: "approvalWaiting",
  open_questions: "openQuestions",
  requirement_without_work: "requirementWithoutWork",
} as const;

const ATTENTION_ICONS: Record<string, { icon: IconName; style: string }> = {
  approval_waiting: {
    icon: "Time",
    style: "bg-[var(--warning-100)] text-[var(--warning-700)]",
  },
  open_questions: { icon: "Help", style: "bg-[var(--info-100)] text-[var(--info-700)]" },
  requirement_without_work: {
    icon: "Attention",
    style: "bg-[var(--danger-100)] text-[var(--danger-700)]",
  },
};

function Kpis({ overview, days }: { overview: OverviewData; days: number }) {
  const { t } = useTranslation();
  const delta = overview.tasks_completed - overview.tasks_completed_previous;
  const percent = coveragePercent(overview.coverage);
  const deltaText =
    delta > 0
      ? t("overview.kpi.tasksUp", { count: delta, days })
      : delta < 0
        ? t("overview.kpi.tasksDown", { count: delta, days })
        : t("overview.kpi.tasksSame", { days });
  return (
    <section
      aria-label={t("overview.title")}
      className="grid gap-4 [grid-template-columns:repeat(auto-fit,minmax(min(100%,190px),1fr))]"
    >
      <Kpi
        label={t("overview.kpi.activeSpecs")}
        tip={t("overview.kpi.activeSpecsTip")}
        value={String(overview.active_specs)}
        sub={t("overview.kpi.activeSpecsSub")}
      />
      <Kpi
        label={t("overview.kpi.tasksCompleted")}
        tip={t("overview.kpi.tasksCompletedTip")}
        value={String(overview.tasks_completed)}
        sub={deltaText}
        tone={delta > 0 ? "up" : delta < 0 ? "down" : "flat"}
      />
      <Kpi
        label={t("overview.kpi.coverage")}
        tip={t("overview.kpi.coverageTip")}
        value={percent === null ? "—" : `${percent}%`}
        sub={
          overview.coverage && percent !== null
            ? t("overview.kpi.coverageSub", {
                covered: overview.coverage.covered,
                total: overview.coverage.total,
                spec: String(overview.coverage.spec_number).padStart(3, "0"),
              })
            : t("overview.kpi.coverageNone")
        }
      />
      <Kpi
        label={t("overview.kpi.openMarkers")}
        tip={t("overview.kpi.openMarkersTip")}
        value={String(overview.open_markers)}
        sub={t("overview.kpi.openMarkersSub")}
      />
      <Kpi
        label={t("overview.kpi.agentsActive")}
        tip={t("overview.kpi.agentsActiveTip")}
        value={String(overview.agents_active)}
        sub={t("overview.kpi.agentsActiveSub")}
      />
    </section>
  );
}

function CompletedChart({ overview }: { overview: OverviewData }) {
  const { t, i18n } = useTranslation();
  const language = i18n.language === "es" ? "es" : "en";
  const chart = chartData(overview.completed_per_day);
  const compact = overview.completed_per_day.length > 14;
  return (
    <section className="flex min-w-0 flex-col gap-3 rounded bg-white p-6 [flex:2_1_560px]">
      <h2 className="m-0 text-lg font-medium leading-[26px]">{t("overview.chart.title")}</h2>
      {chart.total === 0 ? (
        <p className="m-0 py-16 text-center text-sm text-[var(--neutral-500)]">
          {t("overview.chart.empty")}
        </p>
      ) : (
        <BarChart
          label={t("overview.chart.label")}
          labels={chart.days.map((day) => formatShortDay(day, language, compact))}
          series={chart.series}
          max={chart.max}
          height={240}
        />
      )}
    </section>
  );
}

function Attention({ overview, base }: { overview: OverviewData; base: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  return (
    <section className="flex min-w-0 flex-col gap-1 rounded bg-white p-6 [flex:1_1_320px]">
      <div className="mb-2 flex items-center gap-2">
        <h2 className="m-0 flex-1 text-lg font-medium leading-[26px]">
          {t("overview.attention.title")}
        </h2>
        {overview.attention.length > 0 && (
          <span className="h-5 min-w-5 rounded-full bg-[var(--danger-600)] px-1.5 text-center font-numeric text-xs font-medium leading-5 text-white">
            {overview.attention.length}
          </span>
        )}
      </div>
      {overview.attention.length === 0 ? (
        <p className="m-0 py-6 text-sm text-[var(--neutral-500)]">
          {t("overview.attention.empty")}
        </p>
      ) : (
        <ul className="m-0 list-none p-0">
          {overview.attention.map((item) => {
            const kind = isAttentionKind(item.kind) ? item.kind : null;
            const key = kind ? ATTENTION_KEYS[kind] : null;
            const target = attentionPath(base, item);
            const spec = String(item.spec_number).padStart(3, "0");
            const icon = ATTENTION_ICONS[item.kind] ?? ATTENTION_ICONS.open_questions;
            return (
              <li
                key={`${item.kind}-${item.spec_number}`}
                className="flex items-start gap-3 border-t border-[var(--neutral-200)] py-3"
              >
                <span
                  className={cn(
                    "flex size-7 flex-none items-center justify-center rounded-full",
                    icon?.style,
                  )}
                >
                  <Icon name={icon?.icon ?? "Help"} size={16} />
                </span>
                <div className="flex min-w-0 flex-1 flex-col">
                  <span className="text-sm leading-[22px] text-[var(--neutral-700)]">
                    {key === "approvalWaiting"
                      ? t("overview.attention.approvalWaiting", { spec })
                      : key === "openQuestions"
                        ? t("overview.attention.openQuestions", { spec, count: item.count })
                        : key === "requirementWithoutWork"
                          ? t("overview.attention.requirementWithoutWork", {
                              spec,
                              count: item.count,
                            })
                          : item.kind}
                  </span>
                  <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                    {key === "approvalWaiting"
                      ? t("overview.attention.approvalWaitingSub")
                      : item.detail.slice(0, 2).join(" · ")}
                  </span>
                </div>
                {key && target && (
                  <Button
                    size="small"
                    variant="outline"
                    aria-label={`${t(`overview.attention.${key}Action`)} ${spec}`}
                    onClick={() => navigate(target)}
                  >
                    {t(`overview.attention.${key}Action`)}
                  </Button>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

export function Overview() {
  const { project, status, summary, overview, active, activity, days } = useLoaderData() as Awaited<
    ReturnType<typeof loader>
  >;
  const root = useRouteLoaderData("root") as RootData | undefined;
  const { t } = useTranslation();
  const [, setSearchParams] = useSearchParams();
  const navigate = useNavigate();
  useLiveRevalidate(project.address);
  const base = `/p/${project.address}`;
  const owner = project.role === "owner";
  const specWritten = status.specs.some((spec) => spec.number === 1);
  const agentCalled = isAgentClient(summary?.last_activity?.client);
  const gettingStarted = owner && !project.archived && !specWritten;
  const activeSpecs = [
    ...(active.active ? [{ ...active.active, draft: false }] : []),
    ...active.drafts.map((draft) => ({
      number: draft.number,
      slug: draft.slug,
      title: draft.title,
      status: "draft",
      branch: branch(draft.number, draft.slug),
      draft: true,
      current_phase: null,
      tasks_done: 0,
      tasks_total: 0,
    })),
  ];
  return (
    <div className="mx-auto flex max-w-[1280px] flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3 pt-2">
        <h1 className="m-0 flex items-center gap-3 font-mono text-2xl font-medium leading-8">
          {project.name}
          {project.archived && <Tag>{t("card.archived")}</Tag>}
        </h1>
        <Link to={`/o/${project.organization}`} className="text-[13px] text-[var(--neutral-500)]">
          {project.organization}
        </Link>
        {project.repository_url && (
          <a
            href={project.repository_url}
            rel="noreferrer"
            className="font-mono text-[13px] text-[var(--neutral-500)] hover:underline"
          >
            {project.repository_url}
          </a>
        )}
        <span className="flex-1" />
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
        <fieldset className="m-0 flex gap-2 border-0 p-0">
          <legend className="sr-only">{t("overview.period")}</legend>
          {PERIODS.map((period) => (
            <CheckableTag
              key={period}
              size="small"
              checked={days === period}
              onChange={() =>
                setSearchParams(period === 7 ? {} : { days: String(period) }, { replace: true })
              }
            >
              {t("overview.periodOption", { days: period })}
            </CheckableTag>
          ))}
        </fieldset>
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
      <Kpis overview={overview} days={days} />
      <div className="flex flex-wrap items-stretch gap-4">
        <CompletedChart overview={overview} />
        <Attention overview={overview} base={base} />
      </div>
      <div className="flex flex-wrap items-stretch gap-4">
        <section className="flex min-w-0 flex-col gap-1 rounded bg-white p-6 [flex:2_1_560px]">
          <div className="mb-2 flex items-center">
            <h2 className="m-0 flex-1 text-lg font-medium leading-[26px]">
              {t("overview.active.title")}
            </h2>
            <Button
              variant="link"
              aria-label={`${t("overview.active.viewAll")}: ${t("overview.active.title")}`}
              onClick={() => navigate(`${base}/active`)}
            >
              {t("overview.active.viewAll")}
            </Button>
          </div>
          {activeSpecs.length === 0 ? (
            <p className="m-0 py-6 text-sm text-[var(--neutral-500)]">
              {t("overview.active.empty")}
            </p>
          ) : (
            <ul className="m-0 list-none p-0">
              {activeSpecs.map((spec) => (
                <li key={spec.number}>
                  <Link
                    to={`${base}/specs/${spec.number}`}
                    className="-mx-3 grid items-center gap-4 rounded px-3 py-3.5 text-inherit no-underline hover:bg-[var(--neutral-50)] [grid-template-columns:44px_minmax(0,1fr)_200px]"
                  >
                    <span className="font-mono text-lg font-medium text-[var(--neutral-400)]">
                      {String(spec.number).padStart(3, "0")}
                    </span>
                    <span className="flex min-w-0 flex-col gap-0.5">
                      <span className="flex flex-wrap items-center gap-2">
                        <span className="text-[15px] font-medium leading-[22px]">{spec.title}</span>
                        <StatusTag status={specTag(spec.status)}>
                          {t(`spec.statuses.${spec.status as "draft"}`)}
                        </StatusTag>
                      </span>
                      <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                        {spec.draft
                          ? t("overview.active.draftSub", { branch: spec.branch })
                          : spec.branch}
                      </span>
                    </span>
                    {!spec.draft && (
                      <span className="flex flex-col gap-1.5">
                        <span className="flex justify-between text-xs text-[var(--neutral-500)]">
                          <span>
                            {spec.current_phase
                              ? t("overview.active.phase", { phase: spec.current_phase })
                              : ""}
                          </span>
                          <span className="font-numeric">
                            {t("overview.active.progress", {
                              done: spec.tasks_done,
                              total: spec.tasks_total,
                            })}
                          </span>
                        </span>
                        <SpecProgress
                          done={spec.tasks_done}
                          total={spec.tasks_total}
                          label={spec.title}
                        />
                      </span>
                    )}
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </section>
        <section className="flex min-w-0 flex-col gap-1 rounded bg-white p-6 [flex:1_1_320px]">
          <div className="mb-2 flex items-center gap-2">
            <h2 className="m-0 flex-1 text-lg font-medium leading-[26px]">
              {t("overview.recent.title")}
            </h2>
            <Button
              variant="link"
              aria-label={`${t("activity.viewAll")}: ${t("overview.recent.title")}`}
              onClick={() => navigate(`${base}/activity`)}
            >
              {t("activity.viewAll")}
            </Button>
          </div>
          {activity.rows.length === 0 ? (
            <p className="m-0 py-6 text-sm text-[var(--neutral-500)]">
              {t("overview.recent.empty")}
            </p>
          ) : (
            <ul aria-label={t("overview.recent.title")} className="m-0 list-none p-0">
              {activity.rows.map((row) => (
                <ActivityItem key={row.audit_id} row={row} rows={activity.rows} compact />
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
