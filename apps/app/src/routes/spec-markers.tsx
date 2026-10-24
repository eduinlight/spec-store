import { Avatar, Icon } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import { type LoaderFunctionArgs, useLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { type SpecKind, useDate } from "~/lib/format";
import { guard } from "~/lib/session";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const specPath = { ...path, number: Number(params.number) };
  return guard(request, async () =>
    api
      .GET("/api/v1/projects/{org}/{project}/specs/{number}/markers", {
        params: { path: specPath },
      })
      .then(unwrap),
  );
}

const MARKER_TOKENS: Record<string, string> = {
  clarification: "[NEEDS CLARIFICATION]",
  research: "[NEEDS RESEARCH]",
};

export function SpecMarkers() {
  const { markers, resolved } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const date = useDate();
  const last = resolved.reduce<(typeof resolved)[number] | null>(
    (latest, item) => (!latest || item.resolved_by.at > latest.resolved_by.at ? item : latest),
    null,
  );
  const whereOf = (kind: string, heading: string) =>
    `${t(`spec.kinds.${kind as SpecKind}`)} · ${heading}`;

  return (
    <>
      {markers.length === 0 ? (
        <div className="flex flex-col items-center gap-2 rounded bg-white px-6 pb-8 pt-10 text-center">
          <span className="flex size-12 items-center justify-center rounded-full bg-[var(--success-100)] text-[var(--success-700)]">
            <Icon name="Check" size={24} />
          </span>
          <span className="text-base font-medium leading-6">{t("spec.markersTab.emptyTitle")}</span>
          <span className="text-[13px] text-[var(--neutral-500)]">
            {last
              ? `${t("spec.markersTab.allResolved", { count: resolved.length })} ${t("spec.markersTab.lastResolved", { author: last.resolved_by.author, when: date(last.resolved_by.at) })}`
              : t("spec.markersTab.neverAsked")}
          </span>
        </div>
      ) : (
        <section
          aria-label={t("spec.markersTab.open")}
          className="flex flex-col rounded bg-white px-6 pb-4 pt-2"
        >
          <h2 className="m-0 pb-1 pt-3 text-xs font-normal uppercase text-[var(--neutral-500)]">
            {t("spec.markersTab.open")}
          </h2>
          <ul className="m-0 list-none p-0">
            {markers.map((marker) => (
              <li
                key={`${marker.document_kind}-${marker.line}`}
                className="flex flex-col gap-1 border-t border-[var(--neutral-200)] py-3.5 text-[13px] leading-5"
              >
                <span className="text-[var(--neutral-700)]">
                  <span className="font-mono text-xs text-[var(--warning-700)]">
                    {MARKER_TOKENS[marker.kind] ?? marker.kind}
                  </span>{" "}
                  {marker.text}
                </span>
                <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                  {whereOf(marker.document_kind, marker.heading)}
                </span>
              </li>
            ))}
          </ul>
        </section>
      )}
      {resolved.length > 0 && (
        <section
          aria-label={t("spec.markersTab.resolved")}
          className="flex flex-col rounded bg-white px-6 pb-4 pt-2"
        >
          <h2 className="m-0 pb-1 pt-3 text-xs font-normal uppercase text-[var(--neutral-500)]">
            {t("spec.markersTab.resolved")}
          </h2>
          <ul className="m-0 list-none p-0">
            {resolved.map((item) => (
              <li
                key={`${item.document_kind}-${item.resolved_by.revision}-${item.text}`}
                className="grid grid-cols-[28px_minmax(0,1fr)_auto] items-start gap-3 border-t border-[var(--neutral-200)] py-3.5"
              >
                <Avatar name={item.resolved_by.author} size={28} />
                <div className="flex min-w-0 flex-col gap-1 text-[13px] leading-5">
                  <span className="text-[var(--neutral-500)] line-through decoration-[var(--neutral-300)]">
                    <span className="font-mono text-xs">
                      {MARKER_TOKENS[item.kind] ?? item.kind}
                    </span>{" "}
                    {item.text}
                  </span>
                  <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                    {t("spec.markersTab.resolvedBy", {
                      author: item.resolved_by.author,
                      where: whereOf(item.document_kind, item.heading),
                    })}
                    {item.resolved_by.client
                      ? ` · ${t("spec.markersTab.via", { client: item.resolved_by.client })}`
                      : ""}
                  </span>
                </div>
                <span className="whitespace-nowrap text-xs text-[var(--neutral-500)]">
                  {date(item.resolved_by.at)}
                </span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </>
  );
}
