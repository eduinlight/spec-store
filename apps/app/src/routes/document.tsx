import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { Markdown } from "~/components/Markdown";
import { type SpecKind, useDate } from "~/lib/format";
import { guard } from "~/lib/session";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  return guard(request, async () => {
    const constitution = await api
      .GET("/api/v1/projects/{org}/{project}/constitution", { params: { path } })
      .then(unwrap);
    return {
      address: `${path.org}/${path.project}`,
      kind: "constitution",
      number: null,
      ...constitution,
    };
  });
}

export function DocumentView({ body }: { body: string }) {
  const { t } = useTranslation();
  const [raw, setRaw] = useState(false);
  return (
    <div className="grid gap-3">
      <div className="flex gap-2">
        <Button size="sm" variant={raw ? "outline" : "default"} onClick={() => setRaw(false)}>
          {t("spec.rendered")}
        </Button>
        <Button size="sm" variant={raw ? "default" : "outline"} onClick={() => setRaw(true)}>
          {t("spec.raw")}
        </Button>
      </div>
      {raw ? (
        <pre
          data-testid="raw"
          className="overflow-x-auto whitespace-pre-wrap rounded-md bg-muted p-4 text-sm"
        >
          {body}
        </pre>
      ) : (
        <Markdown body={body} />
      )}
    </div>
  );
}

export function Document() {
  const data = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const date = useDate();
  const base = `/p/${data.address}`;
  const history =
    data.number === null
      ? `${base}/history?kind=constitution`
      : `${base}/history?spec=${data.number}&kind=${data.kind}`;
  return (
    <Card>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h1 className="text-xl font-semibold">
          {t(`spec.kinds.${data.kind as SpecKind}`)}
          {data.version ? ` · ${t("project.version", { version: data.version })}` : ""}
        </h1>
        <Link to={history} className="text-sm hover:underline">
          {t("spec.history")}
        </Link>
      </div>
      <p className="text-xs text-muted-foreground">
        {data.revision.author} · {date(data.revision.created_at)} · {data.revision.message}
      </p>
      <DocumentView body={data.body} />
    </Card>
  );
}
