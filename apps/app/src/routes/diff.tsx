import { Card } from "@workspace/ui/components/card";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData } from "react-router";
import { api, unwrap } from "~/api/client";
import { guard } from "~/lib/session";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  const id = Number(params.id);
  return guard(request, async () => {
    const diff = await api
      .GET("/api/v1/projects/{org}/{project}/revisions/{id}/diff", {
        params: { path: { ...path, id } },
      })
      .then(unwrap);
    return { address: `${path.org}/${path.project}`, ...diff };
  });
}

const signs: Record<string, string> = { insert: "+", delete: "-", equal: " " };
const tones: Record<string, string> = {
  insert: "bg-green-500/10",
  delete: "bg-red-500/10",
  equal: "",
};

export function Diff() {
  const data = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const base = `/p/${data.address}`;
  return (
    <Card>
      <h1 className="text-xl font-semibold">
        {t("history.diff")} ·{" "}
        <Link to={`${base}/revisions/${data.revision}`} className="text-primary hover:underline">
          {t("history.revision", { id: data.revision })}
        </Link>
      </h1>
      {data.previous_revision === null && (
        <p className="text-sm text-muted-foreground">{t("history.noPrevious")}</p>
      )}
      <div className="overflow-x-auto rounded-md border font-mono text-xs">
        <table className="w-full">
          <tbody>
            {data.hunks.flatMap((hunk, hunkIndex) =>
              hunk.lines.map((line, lineIndex) => (
                <tr key={`${hunkIndex}-${lineIndex}`} className={tones[line.tag]}>
                  <td className="w-12 select-none px-2 text-right text-muted-foreground">
                    {line.old_line ?? ""}
                  </td>
                  <td className="w-12 select-none px-2 text-right text-muted-foreground">
                    {line.new_line ?? ""}
                  </td>
                  <td className="whitespace-pre px-2">{`${signs[line.tag] ?? " "} ${line.text}`}</td>
                </tr>
              )),
            )}
          </tbody>
        </table>
      </div>
    </Card>
  );
}
