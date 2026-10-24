import { Card } from "@workspace/ui/components/card";
import { FacetedFilter } from "@workspace/ui/patterns";
import { Button, Icon, Input, LockDocument, message } from "@workspace/ui/taptap";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { type LoaderFunctionArgs, useLoaderData, useParams, useSearchParams } from "react-router";
import { api, unwrap } from "~/api/client";
import { AuditTable, ColumnMenu, type HideableColumn } from "~/components/audit-table";
import {
  type AuditSort,
  type AuditView,
  apiQuery,
  exportUrl,
  isFiltered,
  PERIODS,
  type Period,
  parseView,
  type RESULTS,
  viewParams,
} from "~/lib/audit-table";
import { useLiveRevalidate } from "~/lib/live";
import { guard } from "~/lib/session";

export const TEXT_DEBOUNCE_MS = 300;

export async function loader({ request, params }: LoaderFunctionArgs) {
  const query = apiQuery(parseView(new URL(request.url).searchParams));
  return guard(request, async () => {
    if (params.project) {
      const path = { org: params.org ?? "", project: params.project };
      return api
        .GET("/api/v1/projects/{org}/{project}/audit/table", { params: { path, query } })
        .then(unwrap);
    }
    return api
      .GET("/api/v1/orgs/{org}/audit/table", {
        params: { path: { org: params.org ?? "" }, query },
      })
      .then(unwrap);
  });
}

interface Facet {
  value: string;
  count: number;
}

const facetOptions = (
  facet: Facet[],
  selected: string[],
  labelOf: (value: string) => string = (value) => value,
  mono = false,
) => {
  const known = new Set(facet.map((item) => item.value));
  const missing = selected.filter((value) => !known.has(value));
  return [...facet, ...missing.map((value) => ({ value, count: 0 }))].map((item) => ({
    value: item.value,
    label: labelOf(item.value),
    count: item.count,
    mono,
  }));
};

function LiveAudit({ address }: { address: string }) {
  useLiveRevalidate(address);
  return null;
}

export function Audit() {
  const data = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { org, project } = useParams();
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const view = parseView(searchParams);
  const latest = useRef(view);
  latest.current = view;
  const [text, setText] = useState(view.q);
  const committed = useRef(view.q);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [selection, setSelection] = useState<{ search: string; ids: number[] }>({
    search: "",
    ids: [],
  });
  const [hidden, setHidden] = useState<HideableColumn[]>([]);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const search = searchParams.toString();

  const update = (patch: Partial<AuditView>, replace = false) =>
    setSearchParams(viewParams({ ...latest.current, page: 1, ...patch }), { replace });

  useEffect(() => {
    if (view.q !== committed.current) {
      committed.current = view.q;
      setText(view.q);
    }
  }, [view.q]);

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );

  useEffect(() => {
    if (!menuOpen) return;
    const close = (event: MouseEvent | KeyboardEvent) => {
      if (event instanceof KeyboardEvent) {
        if (event.key === "Escape") setMenuOpen(false);
        return;
      }
      if (!menuRef.current?.contains(event.target as Node)) setMenuOpen(false);
    };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", close);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", close);
    };
  }, [menuOpen]);

  const type = (value: string) => {
    setText(value);
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      timer.current = null;
      committed.current = value;
      update({ q: value }, true);
    }, TEXT_DEBOUNCE_MS);
  };

  const reset = () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    committed.current = "";
    setText("");
    update({ q: "", actors: [], clients: [], actions: [], results: [], period: "all" });
  };

  const base = project ? `/api/v1/projects/${org}/${project}` : `/api/v1/orgs/${org}`;
  const ids = (selection.search === search ? selection.ids : []).filter((id) =>
    data.entries.some((entry) => entry.id === id),
  );

  const download = () => {
    const link = document.createElement("a");
    link.href = exportUrl(base, view, ids);
    link.download = "";
    document.body.appendChild(link);
    link.click();
    link.remove();
    message.success(t("audit.exportStarted"));
  };

  const resultLabel = (value: string) =>
    t(`audit.results.${value as (typeof RESULTS)[number]}`, { defaultValue: value });
  const periodLabel = (value: string) => t(`audit.periods.${value as Period}`);
  const facets = data.facets;
  const shared = {
    emptyText: t("audit.noFacet"),
    clearLabel: t("audit.clearFilters"),
  };

  return (
    <Card>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="flex items-center gap-2 text-xl font-semibold">
            <LockDocument size={20} />
            {t("audit.title")}
          </h1>
          <p className="text-sm text-muted-foreground">{t("audit.immutable")}</p>
        </div>
        <Button variant="outline" leftIcon={<Icon name="Download" size={16} />} onClick={download}>
          {ids.length > 0 ? t("audit.exportSelected", { count: ids.length }) : t("audit.export")}
        </Button>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Input
          aria-label={t("audit.filterLabel")}
          placeholder={t("audit.filterPlaceholder")}
          value={text}
          allowClear
          prefix={<Icon name="Search" size={16} />}
          onChange={type}
          style={{ width: 280 }}
        />
        <FacetedFilter
          label={t("audit.actor")}
          searchPlaceholder={t("audit.searchFacet", { facet: t("audit.actor") })}
          options={facetOptions(facets.actors, view.actors)}
          value={view.actors}
          onChange={(actors) => update({ actors })}
          {...shared}
        />
        <FacetedFilter
          label={t("audit.via")}
          searchPlaceholder={t("audit.searchFacet", { facet: t("audit.via") })}
          options={facetOptions(facets.clients, view.clients)}
          value={view.clients}
          onChange={(clients) => update({ clients })}
          {...shared}
        />
        <FacetedFilter
          label={t("audit.action")}
          searchPlaceholder={t("audit.searchFacet", { facet: t("audit.action") })}
          options={facetOptions(facets.actions, view.actions, undefined, true)}
          value={view.actions}
          onChange={(actions) => update({ actions })}
          {...shared}
        />
        <FacetedFilter
          label={t("audit.result")}
          searchPlaceholder={t("audit.searchFacet", { facet: t("audit.result") })}
          options={facetOptions(facets.results, view.results, resultLabel)}
          value={view.results}
          onChange={(results) => update({ results })}
          {...shared}
        />
        <FacetedFilter
          label={t("audit.period")}
          searchPlaceholder={t("audit.searchFacet", { facet: t("audit.period") })}
          options={PERIODS.map((value) => ({ value, label: periodLabel(value) }))}
          value={view.period === "all" ? [] : [view.period]}
          onChange={(next) =>
            update({ period: (next.find((value) => value !== view.period) as Period) ?? "all" })
          }
          {...shared}
        />
        {isFiltered(view) ? (
          <Button variant="ghost" onClick={reset} rightIcon={<Icon name="Close" size={16} />}>
            {t("audit.reset")}
          </Button>
        ) : null}
        <span className="flex-1" />
        <div ref={menuRef}>
          <ColumnMenu
            hidden={hidden}
            onChange={setHidden}
            open={menuOpen}
            onToggleOpen={setMenuOpen}
          />
        </div>
      </div>
      <AuditTable
        entries={data.entries}
        total={data.total}
        page={view.page}
        pageSize={view.pageSize}
        sort={view.sort}
        hidden={hidden}
        showProject={!project}
        selected={ids}
        onSelect={(next) => setSelection({ search, ids: next })}
        onSort={(sort: AuditSort) => update({ sort })}
        onPage={(page) => update({ page })}
        onPageSize={(pageSize) => update({ pageSize })}
      />
      {project ? <LiveAudit address={`${org}/${project}`} /> : null}
    </Card>
  );
}
