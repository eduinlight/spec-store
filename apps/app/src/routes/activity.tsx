import { cn } from "@workspace/ui/lib/utils";
import { Button, CheckableTag, Icon, message } from "@workspace/ui/taptap";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { type LoaderFunctionArgs, useLoaderData, useSearchParams } from "react-router";
import { api, unwrap } from "~/api/client";
import { ActivityItem } from "~/components/activity-item";
import {
  ACTIVITY_FILTERS,
  ACTIVITY_PAGE_SIZE,
  type ActivityFilter,
  type ActivityRow,
  FRESH_ROW_MS,
  filterOf,
  mergeRows,
  oldestId,
} from "~/lib/activity";
import { useLiveRevalidate } from "~/lib/live";
import { guard } from "~/lib/session";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const filter = filterOf(new URL(request.url).searchParams.get("filter"));
  return guard(request, async () => {
    const [project, page] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      api
        .GET("/api/v1/projects/{org}/{project}/activity", {
          params: { path, query: { filter, limit: ACTIVITY_PAGE_SIZE } },
        })
        .then(unwrap),
    ]);
    return { project, page, filter };
  });
}

interface FeedProps {
  org: string;
  slug: string;
  filter: ActivityFilter;
  first: ActivityRow[];
  firstHasMore: boolean;
}

function Feed({ org, slug, filter, first, firstHasMore }: FeedProps) {
  const { t } = useTranslation();
  const [older, setOlder] = useState<ActivityRow[]>([]);
  const [olderHasMore, setOlderHasMore] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(false);
  const [fresh, setFresh] = useState<ReadonlySet<number>>(new Set());
  const seen = useRef<Set<number> | null>(null);
  const rows = mergeRows(first, older);
  const hasMore = olderHasMore ?? firstHasMore;

  useEffect(() => {
    if (seen.current === null) {
      seen.current = new Set(first.map((row) => row.audit_id));
      return;
    }
    const known = seen.current;
    const arrived = first.filter((row) => !known.has(row.audit_id)).map((row) => row.audit_id);
    for (const id of arrived) known.add(id);
    if (arrived.length === 0) return;
    setFresh(new Set(arrived));
    const timer = setTimeout(() => setFresh(new Set()), FRESH_ROW_MS);
    return () => clearTimeout(timer);
  }, [first]);

  const loadMore = async () => {
    setLoading(true);
    try {
      const page = await api
        .GET("/api/v1/projects/{org}/{project}/activity", {
          params: {
            path: { org, project: slug },
            query: { filter, limit: ACTIVITY_PAGE_SIZE, before: oldestId(rows) },
          },
        })
        .then(unwrap);
      setOlder((previous) => mergeRows(previous, page.rows));
      setOlderHasMore(page.has_more);
    } catch {
      message.error(t("activity.loadFailed"));
    } finally {
      setLoading(false);
    }
  };

  if (rows.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 rounded bg-white px-6 pb-10 pt-12 text-center">
        <span className="flex size-12 items-center justify-center rounded-full bg-[var(--primary-100)] text-[var(--primary-700)]">
          <Icon name="Time" size={24} />
        </span>
        <h2 className="m-0 text-base font-medium leading-6">
          {filter === "all" ? t("activity.emptyTitle") : t("activity.emptyFilter")}
        </h2>
        {filter === "all" && (
          <p className="m-0 text-[13px] text-[var(--neutral-500)]">{t("activity.emptyBody")}</p>
        )}
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-4 rounded bg-white px-3 pb-4 pt-1">
      <ul aria-label={t("activity.list")} className="m-0 list-none p-0">
        {rows.map((row) => (
          <ActivityItem key={row.audit_id} row={row} rows={rows} fresh={fresh.has(row.audit_id)} />
        ))}
      </ul>
      {hasMore && (
        <div className="flex justify-center">
          <Button variant="outline" disabled={loading} onClick={() => void loadMore()}>
            {loading ? t("activity.loading") : t("activity.loadMore")}
          </Button>
        </div>
      )}
    </div>
  );
}

export function Activity() {
  const { project, page, filter } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const [, setSearchParams] = useSearchParams();
  const live = useLiveRevalidate(project.address);
  return (
    <div className="mx-auto flex max-w-[1280px] flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3 pt-2">
        <h1 className="m-0 text-2xl font-medium leading-8">{t("activity.title")}</h1>
        <span className="ml-1 text-[13px] text-[var(--neutral-500)]">{t("activity.subtitle")}</span>
        <span className="flex-1" />
        <span
          role="status"
          title={live ? t("activity.liveTip") : t("activity.reconnectingTip")}
          className="flex items-center gap-2 text-[13px] text-[var(--neutral-600)]"
        >
          <span
            aria-hidden="true"
            className={cn(
              "size-2 rounded-full",
              live
                ? "bg-[var(--success-600)] shadow-[0_0_0_3px_var(--success-100)]"
                : "bg-[var(--neutral-400)]",
            )}
          />
          {live ? t("activity.live") : t("activity.reconnecting")}
        </span>
        <fieldset className="m-0 flex gap-2 border-0 p-0">
          <legend className="sr-only">{t("activity.filtersLabel")}</legend>
          {ACTIVITY_FILTERS.map((item) => (
            <CheckableTag
              key={item}
              size="small"
              checked={filter === item}
              onChange={() =>
                setSearchParams(item === "all" ? {} : { filter: item }, { replace: true })
              }
            >
              {t(`activity.filters.${item}`)}
            </CheckableTag>
          ))}
        </fieldset>
      </div>
      <Feed
        key={filter}
        org={project.organization}
        slug={project.slug}
        filter={filter}
        first={page.rows}
        firstHasMore={page.has_more}
      />
    </div>
  );
}
