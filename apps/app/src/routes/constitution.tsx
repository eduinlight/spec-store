import { Alert, LockDocument } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { optional } from "~/api/optional";
import { InlineMarkdown, Markdown } from "~/components/Markdown";
import { parseConstitution, toRoman } from "~/lib/constitution";
import { useDate, useDay } from "~/lib/format";
import { guard } from "~/lib/session";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  return guard(request, async () => {
    const [project, constitution, history] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      optional(
        api.GET("/api/v1/projects/{org}/{project}/constitution", { params: { path } }).then(unwrap),
      ),
      optional(
        api
          .GET("/api/v1/projects/{org}/{project}/revisions", {
            params: { path, query: { kind: "constitution" } },
          })
          .then(unwrap),
      ),
    ]);
    return { project, constitution, revisions: history?.revisions ?? [] };
  });
}

export function Constitution() {
  const { project, constitution, revisions } = useLoaderData() as Awaited<
    ReturnType<typeof loader>
  >;
  const { t } = useTranslation();
  const date = useDate();
  const day = useDay();
  const owner = project.role === "owner";
  const first = revisions[revisions.length - 1];
  const last = revisions[0];
  const parsed = constitution ? parseConstitution(constitution.body) : null;

  return (
    <div className="mx-auto flex max-w-[880px] flex-col gap-4 pt-2">
      {project.archived && (
        <Alert status="info" closable={false}>
          {t("archive.banner")}
        </Alert>
      )}
      <article className="flex flex-col gap-7 rounded bg-white px-14 py-12 shadow-[inset_0_3px_0_var(--neutral-700)] max-sm:px-6">
        <header className="flex flex-col items-center gap-3 text-center">
          <span className="text-xs font-medium uppercase leading-[18px] tracking-[0.16em] text-[var(--neutral-500)]">
            {t("constitution.kicker", { project: project.name })}
          </span>
          <h1 className="m-0 text-3xl font-medium leading-[38px]">
            {parsed?.title ?? t("spec.kinds.constitution")}
          </h1>
          {constitution && (
            <div className="flex flex-wrap justify-center gap-4 text-[13px] text-[var(--neutral-500)]">
              <span className="rounded-sm bg-[var(--neutral-700)] px-2 font-mono text-xs font-medium leading-[18px] text-white">
                {t("constitution.version", { version: constitution.version })}
              </span>
              {first && <span>{t("constitution.ratified", { date: day(first.created_at) })}</span>}
              {last && first && last.id !== first.id && (
                <span>
                  {t("constitution.amended", { date: day(last.created_at), author: last.author })}
                </span>
              )}
            </div>
          )}
          <div className="flex items-center gap-2 text-[13px] text-[var(--neutral-600)]">
            <LockDocument size={16} />
            <span>{owner ? t("constitution.lockOwner") : t("constitution.lockLocked")}</span>
          </div>
        </header>
        {constitution && parsed ? (
          <>
            {parsed.intro && (
              <p className="m-0 text-center text-[15px] leading-[26px] text-[var(--neutral-600)]">
                <InlineMarkdown body={parsed.intro} />
              </p>
            )}
            {parsed.principles.length > 0 ? (
              <ol
                aria-label={t("constitution.principles")}
                className="m-0 flex list-none flex-col p-0"
              >
                {parsed.principles.map((principle) => (
                  <li
                    key={principle.number}
                    className="grid grid-cols-[56px_1fr] gap-4 border-t border-[var(--neutral-200)] py-5"
                  >
                    <span className="font-[family-name:var(--font-numeric)] text-2xl font-medium leading-8 text-[var(--primary-700)]">
                      {toRoman(principle.number)}
                    </span>
                    <div className="flex flex-col gap-1.5">
                      <h2 className="m-0 text-lg font-medium leading-[26px]">{principle.title}</h2>
                      <p className="m-0 text-[15px] leading-[26px] text-[var(--neutral-600)]">
                        <InlineMarkdown body={principle.body} />
                      </p>
                    </div>
                  </li>
                ))}
              </ol>
            ) : (
              <Markdown body={constitution.body} />
            )}
          </>
        ) : (
          <div className="flex flex-col items-center gap-2 border-t border-[var(--neutral-200)] py-12 text-center">
            <span className="text-base font-medium leading-6">{t("constitution.noneTitle")}</span>
            <span className="text-[13px] text-[var(--neutral-500)]">
              {t("constitution.noneBody")}
            </span>
          </div>
        )}
        {revisions.length > 0 && (
          <section
            aria-label={t("constitution.history")}
            className="flex flex-col gap-2 border-t border-[var(--neutral-200)] pt-6"
          >
            <div className="flex items-center gap-3">
              <h2 className="m-0 flex-1 text-base font-medium leading-6">
                {t("constitution.history")}
              </h2>
              <Link
                to={`/p/${project.address}/history?kind=constitution`}
                className="text-[13px] text-[var(--primary-700)] hover:underline"
              >
                {t("constitution.fullHistory")}
              </Link>
            </div>
            <ul className="m-0 list-none p-0">
              {revisions.map((revision, index) => (
                <li
                  key={revision.id}
                  className="grid grid-cols-[56px_minmax(0,1fr)] gap-3 border-t border-[var(--neutral-200)] py-3 text-[13px] leading-5"
                >
                  <Link
                    to={`/p/${project.address}/revisions/${revision.id}`}
                    className="font-mono font-medium text-[var(--primary-700)] no-underline hover:underline"
                  >
                    {revision.version
                      ? t("constitution.version", { version: revision.version })
                      : t("constitution.revision", { n: revisions.length - index })}
                  </Link>
                  <span className="flex flex-col">
                    <span className="text-[var(--neutral-700)]">{revision.message}</span>
                    <span className="text-xs text-[var(--neutral-500)]">
                      {[
                        revision.author,
                        revision.client ? t("constitution.via", { client: revision.client }) : null,
                        date(revision.created_at),
                      ]
                        .filter(Boolean)
                        .join(" · ")}
                    </span>
                  </span>
                </li>
              ))}
            </ul>
          </section>
        )}
      </article>
    </div>
  );
}
