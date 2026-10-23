import { Card } from "@workspace/ui/components/card";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { type SpecKind, useDate } from "~/lib/format";
import { guard } from "~/lib/session";
import { DocumentView } from "./document";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const id = Number(params.id);
  return guard(request, async () => {
    const revision = await api
      .GET("/api/v1/projects/{org}/{project}/revisions/{id}", { params: { path: { ...path, id } } })
      .then(unwrap);
    return { address: `${path.org}/${path.project}`, ...revision };
  });
}

export function Revision() {
  const data = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const date = useDate();
  const base = `/p/${data.address}`;
  return (
    <Card>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h1 className="text-xl font-semibold">
          {t(`spec.kinds.${data.kind as SpecKind}`)} ·{" "}
          {t("history.revision", { id: data.revision.id })}
        </h1>
        <Link to={`${base}/revisions/${data.revision.id}/diff`} className="text-sm hover:underline">
          {t("history.diff")}
        </Link>
      </div>
      <p className="text-xs text-muted-foreground">
        {data.revision.author} · {date(data.revision.created_at)} · {data.revision.message}
      </p>
      {!data.current && (
        <p className="rounded-md border border-dashed p-3 text-sm">
          {t("history.notCurrent")}{" "}
          <Link
            to={`${base}/revisions/${data.current_revision}`}
            className="text-primary hover:underline"
          >
            {t("history.showCurrent")}
          </Link>
        </p>
      )}
      <DocumentView body={data.body} />
    </Card>
  );
}
