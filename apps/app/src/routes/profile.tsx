import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { useTranslation } from "react-i18next";
import {
  type ActionFunctionArgs,
  type LoaderFunctionArgs,
  useFetcher,
  useLoaderData,
} from "react-router";
import { api, succeeded, unwrap } from "~/api/client";
import { attempt, errorOf } from "~/lib/actions";
import { useDate } from "~/lib/format";
import { guard } from "~/lib/session";

export async function loader({ request }: LoaderFunctionArgs) {
  return guard(request, async () => {
    const [me, clients] = await Promise.all([
      api.GET("/api/v1/me").then(unwrap),
      api.GET("/api/v1/me/clients").then(unwrap),
    ]);
    return { me, clients: clients.clients };
  });
}

export async function action({ request }: ActionFunctionArgs) {
  const form = await request.formData();
  const client_id = String(form.get("client_id"));
  return guard(request, () =>
    attempt(async () => {
      succeeded(
        await api.DELETE("/api/v1/me/clients/{client_id}", { params: { path: { client_id } } }),
      );
      return { revoked: true };
    }),
  );
}

export function Profile() {
  const { me, clients } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const fetcher = useFetcher();
  const { t } = useTranslation();
  const date = useDate();
  const error = errorOf(fetcher.data);
  return (
    <div className="mx-auto grid max-w-2xl gap-6">
      <Card>
        <h1 className="text-xl font-semibold">{t("profileView.title")}</h1>
        <p>
          {me.name} · {me.handle}
        </p>
        <p className="text-sm text-muted-foreground">
          {t("profileView.email")}: {me.email}
        </p>
      </Card>
      <Card>
        <h2 className="text-lg font-semibold">{t("profileView.clients")}</h2>
        {error && (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        )}
        {clients.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("profileView.noClients")}</p>
        ) : (
          <ul className="divide-y">
            {clients.map((client) => (
              <li
                key={client.client_id}
                aria-label={client.name}
                className="flex items-center justify-between py-3"
              >
                <div>
                  <p className="font-medium">{client.name}</p>
                  <p className="text-xs text-muted-foreground">
                    {t("profileView.granted", { when: date(client.granted_at) })}
                  </p>
                </div>
                <fetcher.Form method="post">
                  <input type="hidden" name="client_id" value={client.client_id} />
                  <Button type="submit" size="sm" variant="destructive">
                    {t("profileView.revoke")}
                  </Button>
                </fetcher.Form>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}
