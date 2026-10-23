import { Card } from "@workspace/ui/components/card";
import { useTranslation } from "react-i18next";
import { Link, type LoaderFunctionArgs, useLoaderData, useLocation } from "react-router";
import { api, unwrap } from "~/api/client";
import { useDate } from "~/lib/format";
import { guard } from "~/lib/session";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const before = new URL(request.url).searchParams.get("before");
  const query = { before: before === null ? undefined : Number(before) };
  return guard(request, async () => {
    if (params.project) {
      const path = { org: params.org ?? "", project: params.project };
      return api
        .GET("/api/v1/projects/{org}/{project}/audit", { params: { path, query } })
        .then(unwrap);
    }
    return api
      .GET("/api/v1/orgs/{org}/audit", { params: { path: { org: params.org ?? "" }, query } })
      .then(unwrap);
  });
}

export function Audit() {
  const page = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const { t } = useTranslation();
  const date = useDate();
  const location = useLocation();
  const last = page.entries[page.entries.length - 1];
  const paged = new URLSearchParams(location.search).has("before");
  return (
    <Card>
      <h1 className="text-xl font-semibold">{t("audit.title")}</h1>
      <div className="overflow-x-auto">
        <table className="w-full text-left text-sm">
          <thead className="text-muted-foreground">
            <tr>
              <th className="py-2 pr-3">{t("audit.when")}</th>
              <th className="py-2 pr-3">{t("audit.actor")}</th>
              <th className="py-2 pr-3">{t("audit.action")}</th>
              <th className="py-2 pr-3">{t("audit.project")}</th>
              <th className="py-2">{t("audit.target")}</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {page.entries.map((entry) => (
              <tr key={entry.id}>
                <td className="py-2 pr-3">{date(entry.created_at)}</td>
                <td className="py-2 pr-3">{entry.actor}</td>
                <td className="py-2 pr-3 font-mono">{entry.action}</td>
                <td className="py-2 pr-3">{entry.project ?? entry.organization ?? ""}</td>
                <td className="py-2">{entry.target}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <nav className="flex justify-between text-sm">
        {paged ? (
          <Link to={location.pathname} className="hover:underline">
            {t("audit.newer")}
          </Link>
        ) : (
          <span />
        )}
        {page.has_more && last && (
          <Link to={`${location.pathname}?before=${last.id}`} className="hover:underline">
            {t("audit.older")}
          </Link>
        )}
      </nav>
    </Card>
  );
}
