import { Card } from "@workspace/ui/components/card";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { type SpecKind, useDate } from "~/lib/format";
import { guard } from "~/lib/session";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const url = new URL(request.url);
  const kind = url.searchParams.get("kind") ?? "constitution";
  const spec = url.searchParams.get("spec");
  return guard(request, async () => {
    const history = await api
      .GET("/api/v1/projects/{org}/{project}/revisions", {
        params: { path, query: { kind, spec: spec === null ? undefined : Number(spec) } },
      })
      .then(unwrap);
    return { address: `${path.org}/${path.project}`, kind, spec, revisions: history.revisions };
  });
}

export function History() {
  const { address, kind, spec, revisions } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const date = useDate();
  const base = `/p/${address}`;
  const documentName = `${spec ? `${spec.padStart(3, "0")} · ` : ""}${t(`spec.kinds.${kind as SpecKind}`)}`;
  return (
    <Card>
      <h1 className="text-xl font-semibold">{t("history.title", { document: documentName })}</h1>
      <div className="overflow-x-auto">
        <table className="w-full text-left text-sm">
          <thead className="text-muted-foreground">
            <tr>
              <th className="py-2 pr-3">{t("history.revision", { id: "" })}</th>
              <th className="py-2 pr-3">{t("history.author")}</th>
              <th className="py-2 pr-3">{t("history.client")}</th>
              <th className="py-2 pr-3">{t("history.when")}</th>
              <th className="py-2 pr-3">{t("history.message")}</th>
              <th className="py-2">{t("history.diff")}</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {revisions.map((revision, index) => (
              <tr key={revision.id}>
                <td className="py-2 pr-3">
                  <Link
                    to={`${base}/revisions/${revision.id}`}
                    className="text-primary hover:underline"
                  >
                    {t("history.revision", { id: revision.id })}
                  </Link>
                  {revision.version && (
                    <span className="ml-2 text-muted-foreground">v{revision.version}</span>
                  )}
                  {index === 0 && (
                    <span className="ml-2 rounded bg-secondary px-1.5 py-0.5 text-xs">
                      {t("history.current")}
                    </span>
                  )}
                </td>
                <td className="py-2 pr-3">{revision.author}</td>
                <td className="py-2 pr-3">{revision.client ?? "—"}</td>
                <td className="py-2 pr-3">{date(revision.created_at)}</td>
                <td className="py-2 pr-3">{revision.message}</td>
                <td className="py-2">
                  <Link to={`${base}/revisions/${revision.id}/diff`} className="hover:underline">
                    {t("history.diff")}
                  </Link>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  );
}
