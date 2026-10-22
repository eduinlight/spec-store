import { Card } from "@workspace/ui/components/card";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { optional } from "~/api/optional";
import { Markdown } from "~/components/Markdown";
import { SPEC_KINDS, type SpecKind, useDate } from "~/lib/format";
import { guard } from "~/lib/session";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const number = Number(params.number);
  return guard(request, async () => {
    const specPath = { ...path, number };
    const [project, spec, markers, tasks, ...documents] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      api
        .GET("/api/v1/projects/{org}/{project}/specs/{number}", { params: { path: specPath } })
        .then(unwrap),
      api
        .GET("/api/v1/projects/{org}/{project}/specs/{number}/markers", {
          params: { path: specPath },
        })
        .then(unwrap),
      optional(
        api
          .GET("/api/v1/projects/{org}/{project}/specs/{number}/tasks", {
            params: { path: specPath },
          })
          .then(unwrap),
      ),
      ...SPEC_KINDS.map((kind) =>
        optional(
          api
            .GET("/api/v1/projects/{org}/{project}/specs/{number}/documents/{kind}", {
              params: { path: { ...specPath, kind } },
            })
            .then(unwrap),
        ),
      ),
    ]);
    return {
      project,
      spec,
      markers: markers.markers,
      tasks,
      documents: SPEC_KINDS.map((kind, i) => ({ kind, document: documents[i] ?? null })),
    };
  });
}

export function Spec() {
  const { project, spec, markers, tasks, documents } = useLoaderData() as Awaited<
    ReturnType<typeof loader>
  >;
  const { t } = useTranslation();
  const date = useDate();
  const base = `/p/${project.address}`;
  return (
    <div className="grid gap-6">
      <div>
        <p className="text-sm text-muted-foreground">
          <Link to={base} className="hover:underline">
            {project.name}
          </Link>
        </p>
        <h1 className="text-2xl font-semibold">{spec.branch}</h1>
        <p className="text-sm text-muted-foreground">
          {t("spec.status")}: {t(`spec.statuses.${spec.status as "draft"}`)} · {t("spec.branch")}:{" "}
          {spec.branch}
        </p>
      </div>
      {documents.map(({ kind, document }) => (
        <Card key={kind}>
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold">{t(`spec.kinds.${kind as SpecKind}`)}</h2>
            {document && (
              <div className="flex gap-3 text-sm">
                <Link to={`${base}/specs/${spec.number}/${kind}`} className="hover:underline">
                  {t("spec.raw")}
                </Link>
                <Link
                  to={`${base}/history?spec=${spec.number}&kind=${kind}`}
                  className="hover:underline"
                >
                  {t("spec.history")}
                </Link>
              </div>
            )}
          </div>
          {document ? (
            <>
              <p className="text-xs text-muted-foreground">
                {document.revision.author} · {date(document.revision.created_at)} ·{" "}
                {document.revision.message}
              </p>
              <Markdown body={document.body} />
            </>
          ) : (
            <p className="text-sm text-muted-foreground">{t("spec.missing")}</p>
          )}
        </Card>
      ))}
      {tasks && (
        <Card>
          <h2 className="text-lg font-semibold">{t("spec.tasks")}</h2>
          <div className="overflow-x-auto">
            <table aria-label={t("spec.tasks")} className="w-full text-left text-sm">
              <thead className="text-muted-foreground">
                <tr>
                  <th className="py-2 pr-3">{t("spec.task")}</th>
                  <th className="py-2 pr-3">{t("spec.phase")}</th>
                  <th className="py-2 pr-3">{t("spec.parallel")}</th>
                  <th className="py-2 pr-3">{t("spec.frs")}</th>
                  <th className="py-2 pr-3">{t("spec.task")}</th>
                  <th className="py-2 pr-3">{t("spec.doneWhen")}</th>
                  <th className="py-2 pr-3">{t("spec.done")}</th>
                  <th className="py-2">{t("spec.commit")}</th>
                </tr>
              </thead>
              <tbody className="divide-y">
                {tasks.tasks.map((task) => (
                  <tr key={task.id}>
                    <td className="py-2 pr-3 font-mono">{task.id}</td>
                    <td className="py-2 pr-3">{task.phase}</td>
                    <td className="py-2 pr-3">{task.parallel ? "P" : ""}</td>
                    <td className="py-2 pr-3">{task.frs.join(", ")}</td>
                    <td className="py-2 pr-3">{task.description}</td>
                    <td className="py-2 pr-3">{task.done_when}</td>
                    <td className="py-2 pr-3">{task.done ? "✓" : ""}</td>
                    <td className="py-2 font-mono">
                      {task.commit &&
                        (project.repository_url ? (
                          <a
                            href={`${project.repository_url.replace(/\/$/, "")}/commit/${task.commit}`}
                            className="text-primary hover:underline"
                            rel="noreferrer"
                          >
                            {task.commit}
                          </a>
                        ) : (
                          task.commit
                        ))}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      )}
      <Card>
        <h2 className="text-lg font-semibold">
          {t("spec.markers")} ({markers.length})
        </h2>
        {markers.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("spec.noMarkers")}</p>
        ) : (
          <ul className="grid gap-2 text-sm">
            {markers.map((marker) => (
              <li key={`${marker.document_kind}-${marker.line}`}>
                <span className="font-medium">
                  {t(`spec.markerKinds.${marker.kind as "research"}`)}
                </span>{" "}
                <span className="text-muted-foreground">
                  {t(`spec.kinds.${marker.document_kind as SpecKind}`)} · {marker.heading}
                </span>
                <p>{marker.text}</p>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}
