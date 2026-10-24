import { cn } from "@workspace/ui/lib/utils";
import { Select, type SelectValue } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import {
  Link,
  type LoaderFunctionArgs,
  useLoaderData,
  useNavigate,
  useRouteLoaderData,
} from "react-router";
import { api, unwrap } from "~/api/client";
import { optional } from "~/api/optional";
import { SPEC_KINDS, type SpecKind, useDate } from "~/lib/format";
import { guard } from "~/lib/session";
import { projectParams } from "./project";
import type { SpecLayoutData } from "./spec";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const number = Number(params.number);
  const url = new URL(request.url);
  const requested = url.searchParams.get("kind");
  const kind = SPEC_KINDS.find((item) => item === requested) ?? "spec";
  const wanted = Number(url.searchParams.get("revision"));
  return guard(request, async () => {
    const history = await optional(
      api
        .GET("/api/v1/projects/{org}/{project}/revisions", {
          params: { path, query: { kind, spec: number } },
        })
        .then(unwrap),
    );
    const revisions = history?.revisions ?? [];
    const selected = revisions.find((revision) => revision.id === wanted) ?? revisions[0] ?? null;
    const diff = selected
      ? await api
          .GET("/api/v1/projects/{org}/{project}/revisions/{id}/diff", {
            params: { path: { ...path, id: selected.id } },
          })
          .then(unwrap)
      : null;
    return { address: `${path.org}/${path.project}`, number, kind, revisions, selected, diff };
  });
}

const SIGNS: Record<string, string> = { insert: "+", delete: "−", equal: " " };
const TONES: Record<string, string> = {
  insert: "bg-[var(--success-100)] text-[var(--success-700)]",
  delete: "bg-[var(--danger-100)] text-[var(--danger-700)]",
  equal: "text-[var(--neutral-700)]",
};

export function SpecHistory() {
  const { address, number, kind, revisions, selected, diff } = useLoaderData() as Awaited<
    ReturnType<typeof loader>
  >;
  const layout = useRouteLoaderData("spec") as SpecLayoutData;
  const { t } = useTranslation();
  const date = useDate();
  const navigate = useNavigate();
  const base = `/p/${address}/specs/${number}/history`;
  const ordinalOf = (id: number | null | undefined): number => {
    const index = revisions.findIndex((revision) => revision.id === id);
    return index < 0 ? 0 : revisions.length - index;
  };
  const kinds = layout.card
    ? SPEC_KINDS.filter((item) => layout.card?.documents.includes(item) || item === kind)
    : SPEC_KINDS;
  const lines = diff?.hunks.flatMap((hunk) => hunk.lines) ?? [];
  const added = lines.filter((line) => line.tag === "insert").length;
  const removed = lines.filter((line) => line.tag === "delete").length;
  const file = `${kind}.md`;

  return (
    <div className="grid items-start gap-4 [grid-template-columns:300px_minmax(0,1fr)] max-md:grid-cols-1">
      <div className="flex flex-col gap-2 rounded bg-white p-2">
        <Select
          aria-label={t("spec.historyTab.documentLabel")}
          width="100%"
          value={kind}
          options={kinds.map((item) => ({
            value: item,
            label: t(`spec.kinds.${item as SpecKind}`),
          }))}
          onChange={(value: SelectValue) => navigate(`${base}?kind=${value}`)}
        />
        {revisions.length === 0 ? (
          <p className="m-0 px-3 py-6 text-center text-[13px] text-[var(--neutral-500)]">
            {t("spec.historyTab.none")}
          </p>
        ) : (
          <ul aria-label={t("spec.historyTab.revisions")} className="m-0 list-none p-0">
            {revisions.map((revision, index) => (
              <li key={revision.id}>
                <Link
                  to={`${base}?kind=${kind}&revision=${revision.id}`}
                  aria-current={revision.id === selected?.id ? "true" : undefined}
                  className={cn(
                    "flex gap-2.5 rounded px-3 py-2.5 text-inherit no-underline hover:bg-[var(--neutral-50)]",
                    revision.id === selected?.id && "bg-[var(--primary-50)]",
                  )}
                >
                  <span
                    className={cn(
                      "w-[34px] font-mono text-[13px] font-medium leading-5",
                      revision.id === selected?.id
                        ? "text-[var(--primary-700)]"
                        : "text-[var(--neutral-500)]",
                    )}
                  >
                    {t("spec.historyTab.revision", { n: revisions.length - index })}
                  </span>
                  <span className="flex min-w-0 flex-1 flex-col text-[13px] leading-5">
                    <span>{revision.message}</span>
                    <span className="text-xs text-[var(--neutral-500)]">
                      {[revision.author, revision.client, date(revision.created_at)]
                        .filter(Boolean)
                        .join(" · ")}
                    </span>
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className="min-w-0 rounded bg-white">
        {selected && diff ? (
          <>
            <div className="flex flex-wrap items-center gap-3 border-b border-[var(--neutral-200)] px-5 py-3">
              <span className="flex-1 text-sm font-medium leading-[22px]">
                {typeof diff.previous_revision !== "number"
                  ? `${t("spec.historyTab.revision", { n: ordinalOf(selected.id) })} · ${t("spec.historyTab.firstRevision", { file })}`
                  : t("spec.historyTab.diffTitle", {
                      from: t("spec.historyTab.revision", { n: ordinalOf(diff.previous_revision) }),
                      to: t("spec.historyTab.revision", { n: ordinalOf(selected.id) }),
                      file,
                    })}
              </span>
              <span className="text-xs text-[var(--success-700)]">+{added}</span>
              <span className="text-xs text-[var(--danger-700)]">−{removed}</span>
              <Link
                to={`/p/${address}/revisions/${selected.id}`}
                className="text-xs text-[var(--primary-700)] hover:underline"
              >
                {t("spec.historyTab.view")}
              </Link>
            </div>
            {lines.length === 0 ? (
              <p className="m-0 px-5 py-8 text-center text-[13px] text-[var(--neutral-500)]">
                {t("spec.historyTab.noChanges")}
              </p>
            ) : (
              <div className="overflow-x-auto font-mono text-[13px] leading-[22px]">
                {lines.map((line) => (
                  <div
                    key={`${line.old_line ?? "-"}:${line.new_line ?? "-"}:${line.tag}`}
                    className={cn("grid grid-cols-[40px_20px_1fr]", TONES[line.tag])}
                  >
                    <span className="pr-2 text-right text-[var(--neutral-400)]">
                      {line.new_line ?? line.old_line ?? ""}
                    </span>
                    <span>{SIGNS[line.tag] ?? " "}</span>
                    <span className="whitespace-pre-wrap pr-4">{line.text}</span>
                  </div>
                ))}
              </div>
            )}
          </>
        ) : (
          <p className="m-0 px-5 py-12 text-center text-sm text-[var(--neutral-500)]">
            {t("spec.historyTab.none")}
          </p>
        )}
      </div>
    </div>
  );
}
