import { Button, Icon, Tabs, type TabsKey } from "@workspace/ui/taptap";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { type LoaderFunctionArgs, useLoaderData, useNavigate } from "react-router";
import { api, unwrap } from "~/api/client";
import { optional } from "~/api/optional";
import { Markdown } from "~/components/Markdown";
import { useDate } from "~/lib/format";
import { headingsOf } from "~/lib/markdown";
import { guard } from "~/lib/session";
import { type DocumentTab, isDocumentTab } from "~/lib/spec-tabs";
import { taskIdsByRequirement } from "~/lib/spec-trace";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const number = Number(params.number);
  const kind = params.kind ?? "spec";
  if (!isDocumentTab(kind)) throw new Response("Not found", { status: 404 });
  return guard(request, async () => {
    const specPath = { ...path, number };
    const [document, readiness, revisions, traceability] = await Promise.all([
      optional(
        api
          .GET("/api/v1/projects/{org}/{project}/specs/{number}/documents/{kind}", {
            params: { path: { ...specPath, kind } },
          })
          .then(unwrap),
      ),
      api
        .GET("/api/v1/projects/{org}/{project}/specs/{number}/readiness", {
          params: { path: specPath },
        })
        .then(unwrap),
      optional(
        api
          .GET("/api/v1/projects/{org}/{project}/revisions", {
            params: { path, query: { kind, spec: number } },
          })
          .then(unwrap),
      ),
      kind === "spec"
        ? optional(
            api
              .GET("/api/v1/projects/{org}/{project}/specs/{number}/traceability", {
                params: { path: specPath },
              })
              .then(unwrap),
          )
        : null,
    ]);
    return {
      address: `${path.org}/${path.project}`,
      number,
      kind: kind as DocumentTab,
      document,
      readiness,
      revisions: revisions?.revisions ?? [],
      requirements: traceability?.requirements ?? [],
    };
  });
}

type DocumentData = Awaited<ReturnType<typeof loader>>;

const VIEW_MODES = ["rendered", "raw"] as const;
type ViewMode = (typeof VIEW_MODES)[number];

function Readiness({ readiness }: { readiness: DocumentData["readiness"] }) {
  const { t } = useTranslation();
  const done = readiness.items.filter((item) => item.ok).length;
  const detailOf = (item: DocumentData["readiness"]["items"][number]): string => {
    switch (item.key) {
      case "open_markers_resolved":
        return item.ok
          ? t("spec.readinessDetails.noneOpen")
          : t("spec.readinessDetails.open", { count: item.count ?? 0 });
      case "requirements_covered":
        return (item.total ?? 0) === 0
          ? t("spec.readinessDetails.noRequirements")
          : t("spec.readinessDetails.covered", {
              covered: item.count ?? 0,
              total: item.total ?? 0,
            });
      case "tasks_written":
        return item.ok
          ? t("spec.readinessDetails.tasks", { count: item.count ?? 0 })
          : t("spec.readinessDetails.missing");
      default:
        return item.ok ? t("spec.readinessDetails.present") : t("spec.readinessDetails.missing");
    }
  };
  return (
    <section
      aria-label={t("spec.readiness")}
      className="flex flex-col gap-2.5 rounded bg-white px-5 py-4 text-[13px] leading-5"
    >
      <div className="flex items-center">
        <h2 className="m-0 flex-1 text-xs font-normal uppercase text-[var(--neutral-500)]">
          {t("spec.readiness")}
        </h2>
        <span
          className={
            readiness.ready
              ? "text-xs font-medium text-[var(--success-700)]"
              : "text-xs font-medium text-[var(--warning-700)]"
          }
        >
          {readiness.ready
            ? t("spec.readinessReady")
            : t("spec.readinessProgress", { done, total: readiness.items.length })}
        </span>
      </div>
      <ul className="m-0 flex list-none flex-col gap-2.5 p-0">
        {readiness.items.map((item) => (
          <li key={item.key} className="flex items-start gap-2">
            <span
              className={
                item.ok
                  ? "mt-0.5 flex text-[var(--success-600)]"
                  : "mt-0.5 flex text-[var(--warning-600)]"
              }
            >
              <Icon name={item.ok ? "SuccessFill" : "AttentionFill"} size={16} />
            </span>
            <span className="flex flex-col">
              <span className="text-[var(--neutral-700)]">
                {t(`spec.readinessLabels.${item.key as "plan_written"}`)}
              </span>
              <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                {detailOf(item)}
              </span>
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function SpecDocument() {
  const data = useLoaderData() as DocumentData;
  const { document, kind, readiness, revisions, requirements } = data;
  const { t } = useTranslation();
  const date = useDate();
  const navigate = useNavigate();
  const [mode, setMode] = useState<ViewMode>("rendered");
  const headings = document ? headingsOf(document.body) : [];
  const tips = new Map(
    [...taskIdsByRequirement(requirements)].map(([id, tasks]) => [
      id,
      tasks.length > 0
        ? t("spec.referencedBy", { tasks: tasks.join(", ") })
        : t("spec.notReferenced"),
    ]),
  );
  const position = document
    ? revisions.length -
      Math.max(
        revisions.findIndex((revision) => revision.id === document.revision.id),
        0,
      )
    : 0;
  const meta = document
    ? [
        t("spec.revisionLine", { n: position, total: revisions.length }),
        t("spec.revisionBy", { author: document.revision.author }),
        ...(document.revision.client
          ? [t("spec.revisionVia", { client: document.revision.client })]
          : []),
        date(document.revision.created_at),
      ].join(" · ")
    : "";

  return (
    <div className="flex flex-wrap items-start gap-4">
      <div className="min-w-0 flex-[1_1_520px] rounded bg-white">
        {document ? (
          <>
            <div className="flex flex-wrap items-center gap-3 border-b border-[var(--neutral-200)] px-6 py-3">
              <span className="flex text-[var(--neutral-500)]">
                <Icon name="Time" size={16} />
              </span>
              <span className="flex-1 text-[13px] leading-5 text-[var(--neutral-600)]">{meta}</span>
              <Tabs
                type="card"
                size="small"
                items={VIEW_MODES.map((value) => ({ value, label: t(`spec.${value}`) }))}
                value={mode}
                onChange={(value: TabsKey) => setMode(value as ViewMode)}
              />
              <Button
                variant="outline"
                size="small"
                onClick={() =>
                  navigate(`/p/${data.address}/specs/${data.number}/history?kind=${kind}`)
                }
              >
                {t("spec.compare")}
              </Button>
            </div>
            {mode === "raw" ? (
              <pre
                data-testid="raw"
                className="m-0 overflow-x-auto whitespace-pre-wrap px-8 py-6 font-mono text-[13px] leading-[22px] text-[var(--neutral-700)]"
              >
                {document.body}
              </pre>
            ) : (
              <article className="max-w-[760px] px-10 pb-12 pt-8 text-base leading-7 text-[var(--neutral-700)]">
                <Markdown body={document.body} requirementTips={tips} />
              </article>
            )}
          </>
        ) : (
          <p className="m-0 px-6 py-12 text-center text-sm text-[var(--neutral-500)]">
            {t("spec.notWritten", { document: t(`spec.kinds.${kind}`) })}
          </p>
        )}
      </div>
      <div className="sticky top-0 flex max-w-[320px] flex-[1_0_240px] flex-col gap-4">
        {headings.length > 0 && (
          <nav
            aria-label={t("spec.onThisPage")}
            className="flex flex-col gap-2 rounded bg-white px-5 py-4 text-[13px] leading-5"
          >
            <span className="text-xs uppercase text-[var(--neutral-500)]">
              {t("spec.onThisPage")}
            </span>
            {headings.map((heading) => (
              <a
                key={heading.id}
                href={`#${heading.id}`}
                className={
                  heading.level === 3
                    ? "pl-3 text-[var(--neutral-600)] no-underline hover:text-[var(--primary-700)]"
                    : "text-[var(--neutral-600)] no-underline hover:text-[var(--primary-700)]"
                }
              >
                {heading.text}
              </a>
            ))}
          </nav>
        )}
        <Readiness readiness={readiness} />
      </div>
    </div>
  );
}
