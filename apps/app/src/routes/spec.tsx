import { Alert, Breadcrumb, StatusTag, Steps, Tabs, type TabsKey } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import {
  type LoaderFunctionArgs,
  Outlet,
  type ShouldRevalidateFunctionArgs,
  useLoaderData,
  useLocation,
  useNavigate,
} from "react-router";
import { api, unwrap } from "~/api/client";
import { specNumber } from "~/lib/format";
import { guard } from "~/lib/session";
import { lifecycleIndex, SPEC_STATUSES, specTag } from "~/lib/spec-status";
import { SPEC_TABS, type SpecTab, tabOf, tabPath } from "~/lib/spec-tabs";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const number = Number(params.number);
  return guard(request, async () => {
    const [project, spec, cards] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      api
        .GET("/api/v1/projects/{org}/{project}/specs/{number}", {
          params: { path: { ...path, number } },
        })
        .then(unwrap),
      api.GET("/api/v1/projects/{org}/{project}/specs/cards", { params: { path } }).then(unwrap),
    ]);
    return { project, spec, card: cards.specs.find((card) => card.number === number) ?? null };
  });
}

export const shouldRevalidate = ({
  currentParams,
  nextParams,
  formMethod,
}: ShouldRevalidateFunctionArgs) =>
  formMethod !== undefined ||
  currentParams.org !== nextParams.org ||
  currentParams.project !== nextParams.project ||
  currentParams.number !== nextParams.number;

export type SpecLayoutData = Awaited<ReturnType<typeof loader>>;

export function Spec() {
  const { project, spec, card } = useLoaderData() as SpecLayoutData;
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const projectBase = `/p/${project.address}`;
  const specBase = `${projectBase}/specs/${spec.number}`;
  const current = tabOf(location.pathname, specBase);
  const archived = spec.status === "archived";

  const labelOf = (tab: SpecTab): string => {
    const label = t(`spec.tabs.${tab}`);
    if (tab === "tasks" && card && card.tasks_total > 0) {
      return `${label} · ${card.tasks_done}/${card.tasks_total}`;
    }
    if (tab === "markers" && card) return `${label} · ${card.open_markers}`;
    return label;
  };

  return (
    <div className="mx-auto flex max-w-[1280px] flex-col gap-4">
      <Breadcrumb
        items={[
          { label: project.name, onClick: () => navigate(projectBase) },
          { label: t("shell.nav.specs"), onClick: () => navigate(`${projectBase}/specs`) },
          { label: specNumber(spec.number) },
        ]}
      />
      <div className="flex flex-col gap-4 rounded bg-white px-6 pb-0 pt-5">
        <div className="flex flex-wrap items-center gap-3">
          <span className="font-mono text-2xl font-medium leading-8 text-[var(--neutral-500)]">
            {specNumber(spec.number)}
          </span>
          <h1 className="m-0 text-2xl font-medium leading-8">{card?.title ?? spec.branch}</h1>
          <StatusTag status={specTag(spec.status)} size="medium">
            {t(`spec.statuses.${spec.status as "draft"}`)}
          </StatusTag>
          <span className="flex items-center gap-1 font-mono text-[13px] leading-5 text-[var(--neutral-500)]">
            <span aria-hidden="true">⑂</span>
            {spec.branch}
          </span>
        </div>
        <Steps
          style={{ maxWidth: 640 }}
          current={lifecycleIndex(spec.status)}
          items={SPEC_STATUSES.map((status) => ({ title: t(`spec.statuses.${status}`) }))}
        />
        <div className="-mx-6 overflow-x-auto px-6">
          <Tabs
            bordered={false}
            style={{ borderBottom: "none" }}
            items={SPEC_TABS.map((tab) => ({
              value: tab,
              label: labelOf(tab),
              disabled:
                tab === "validation" && card !== null && !card.documents.includes("validation"),
            }))}
            value={current}
            onChange={(value: TabsKey) => navigate(tabPath(specBase, value as SpecTab))}
          />
        </div>
      </div>
      {project.archived && (
        <Alert status="info" closable={false}>
          {t("archive.banner")}
        </Alert>
      )}
      {archived && !project.archived && (
        <Alert status="info" closable={false}>
          {t("spec.archivedBanner")}
        </Alert>
      )}
      <div role="tabpanel" aria-label={t(`spec.tabs.${current}`)} className="flex flex-col gap-4">
        <Outlet />
      </div>
    </div>
  );
}
