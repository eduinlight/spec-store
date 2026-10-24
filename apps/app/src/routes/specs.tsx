import { SpecCard, type SpecCardDocument } from "@workspace/ui/patterns";
import { Alert, Button, CheckableTag, Input, Select, type SelectValue } from "@workspace/ui/taptap";
import { type MouseEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { type LoaderFunctionArgs, useLoaderData, useNavigate, useParams } from "react-router";
import { api, unwrap } from "~/api/client";
import { SPEC_KINDS, specNumber, useDate, useMonth } from "~/lib/format";
import { guard } from "~/lib/session";
import {
  activeNumber,
  countByStatus,
  filterSpecs,
  PAGE_SIZE,
  SORTS,
  type SpecCardData,
  type SpecSort,
  STATUS_FILTERS,
  type StatusFilter,
  sortSpecs,
} from "~/lib/spec-list";
import { specTag } from "~/lib/spec-status";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  return guard(request, async () => {
    const [project, cards] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      api.GET("/api/v1/projects/{org}/{project}/specs/cards", { params: { path } }).then(unwrap),
    ]);
    return { project, specs: cards.specs };
  });
}

const DOCUMENT_KEYS: Record<(typeof SPEC_KINDS)[number], string> = {
  spec: "S",
  plan: "P",
  tasks: "T",
  research: "R",
  validation: "V",
};

export function isPlainClick(event: MouseEvent<HTMLElement>): boolean {
  return !(event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || event.button !== 0);
}

export function Specs() {
  const { project, specs } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const date = useDate();
  const month = useMonth();
  const navigate = useNavigate();
  const { org = "", project: slug = "" } = useParams();
  const [status, setStatus] = useState<StatusFilter>("all");
  const [sort, setSort] = useState<SpecSort>("newest");
  const [limit, setLimit] = useState(PAGE_SIZE);
  const [query, setQuery] = useState("");
  const base = `/p/${org}/${slug}`;

  const counts = countByStatus(specs);
  const matching = sortSpecs(filterSpecs(specs, status, query), sort);
  const visible = matching.slice(0, limit);
  const remaining = matching.length - visible.length;
  const active = activeNumber(specs);

  const documentsOf = (card: SpecCardData): SpecCardDocument[] =>
    SPEC_KINDS.map((kind) => ({
      key: DOCUMENT_KEYS[kind],
      title: t(`spec.kinds.${kind}`),
      written: card.documents.includes(kind),
    }));

  return (
    <div className="mx-auto flex max-w-[1080px] flex-col gap-4 pt-2">
      <div className="flex flex-wrap items-center gap-3">
        <h1 className="m-0 text-2xl font-medium leading-8">{t("specs.title")}</h1>
        <span className="text-[13px] text-[var(--neutral-500)]">
          {t("specs.total", { count: specs.length })}
        </span>
      </div>
      {project.archived && (
        <Alert status="info" closable={false}>
          {t("archive.banner")}
        </Alert>
      )}
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <Input
          aria-label={t("specs.search")}
          placeholder={t("specs.search")}
          value={query}
          onChange={(value) => {
            setQuery(value);
            setLimit(PAGE_SIZE);
          }}
          allowClear
          style={{ width: 280 }}
        />
        <div className="flex flex-wrap gap-1.5">
          {STATUS_FILTERS.map((filter) => (
            <CheckableTag
              key={filter}
              checked={status === filter}
              onChange={() => {
                setStatus(filter);
                setLimit(PAGE_SIZE);
              }}
            >
              {`${t(`specs.filters.${filter}`)} ${counts[filter]}`}
            </CheckableTag>
          ))}
        </div>
        <div className="flex-1" />
        <Select
          aria-label={t("specs.sortLabel")}
          width={200}
          value={sort}
          options={SORTS.map((value) => ({ value, label: t(`specs.sorts.${value}`) }))}
          onChange={(value: SelectValue) => {
            setSort(value as SpecSort);
            setLimit(PAGE_SIZE);
          }}
        />
      </div>
      {specs.length === 0 ? (
        <div className="rounded bg-white px-6 py-14 text-center text-sm text-[var(--neutral-500)]">
          {t("specs.none")}
        </div>
      ) : visible.length === 0 ? (
        <div className="rounded bg-white px-6 py-14 text-center text-sm text-[var(--neutral-500)]">
          {t("specs.noMatch")}
        </div>
      ) : (
        <ul aria-label={t("specs.list")} className="m-0 flex list-none flex-col gap-2.5 p-0">
          {visible.map((card) => {
            const href = `${base}/specs/${card.number}`;
            return (
              <li key={card.number}>
                <SpecCard
                  href={href}
                  onClick={(event) => {
                    if (!isPlainClick(event)) return;
                    event.preventDefault();
                    navigate(href);
                  }}
                  number={specNumber(card.number)}
                  title={card.title}
                  status={specTag(card.status)}
                  statusLabel={t(`spec.statuses.${card.status as "draft"}`)}
                  createdLabel={month(card.created_at)}
                  summary={card.summary}
                  slug={card.slug}
                  documents={documentsOf(card)}
                  requirementCount={card.requirement_count}
                  updatedLabel={date(card.updated_at)}
                  contributors={card.contributors}
                  tasksDone={card.tasks_done}
                  tasksTotal={card.tasks_total}
                  openQuestions={card.open_markers}
                  openQuestionsTip={t("specs.questionsTip")}
                  archived={card.status === "archived"}
                  accent={
                    card.status === "draft"
                      ? "draft"
                      : card.number === active && card.status !== "archived"
                        ? "active"
                        : "none"
                  }
                />
              </li>
            );
          })}
        </ul>
      )}
      {specs.length > 0 && (
        <div className="flex flex-col items-center gap-2 pb-4 pt-2">
          <span className="text-xs text-[var(--neutral-500)]">
            {t("specs.showing", { shown: visible.length, total: matching.length })}
          </span>
          {remaining > 0 && (
            <Button variant="outline" onClick={() => setLimit(limit + PAGE_SIZE)}>
              {t("specs.more", { count: Math.min(PAGE_SIZE, remaining) })}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
