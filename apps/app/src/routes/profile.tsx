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
import { useConfirm } from "~/shell/confirm";
import { useOutcomeToast } from "~/shell/outcome";

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
      return { done: "revoke" };
    }),
  );
}

export function Profile() {
  const { me, clients } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const fetcher = useFetcher();
  const { t } = useTranslation();
  const date = useDate();
  const error = errorOf(fetcher.data);
  const confirm = useConfirm();
  useOutcomeToast(fetcher, () => t("toast.accessRevoked"));
  const revoke = async (clientId: string, name: string) => {
    const confirmed = await confirm({
      title: t("confirm.revokeAccessTitle", { agent: name }),
      body: t("confirm.revokeAccessBody"),
      confirmText: t("profileView.revoke"),
    });
    if (confirmed) {
      fetcher.submit({ client_id: clientId }, { method: "post" });
    }
  };
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
                <Button
                  type="button"
                  size="sm"
                  variant="destructive"
                  onClick={() => void revoke(client.client_id, client.name)}
                >
                  {t("profileView.revoke")}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}
