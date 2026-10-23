import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
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

export async function loader({ request, params }: LoaderFunctionArgs) {
  const org = params.org ?? "";
  return guard(request, async () => {
    const invitations = await api
      .GET("/api/v1/orgs/{org}/invitations", { params: { path: { org } } })
      .then(unwrap);
    return { org, invitations: invitations.invitations };
  });
}

export async function action({ request, params }: ActionFunctionArgs) {
  const org = params.org ?? "";
  const form = await request.formData();
  const intent = String(form.get("intent"));
  return guard(request, () =>
    attempt(async () => {
      if (intent === "invite") {
        const email = String(form.get("email") ?? "").trim();
        const language = String(form.get("language") ?? "en");
        const created = unwrap(
          await api.POST("/api/v1/orgs/{org}/invitations", {
            params: { path: { org } },
            body: { email, language },
          }),
        );
        return { sent: created.email };
      }
      const id = Number(form.get("id"));
      if (intent === "resend") {
        succeeded(
          await api.POST("/api/v1/orgs/{org}/invitations/{id}/resend", {
            params: { path: { org, id } },
          }),
        );
      } else {
        succeeded(
          await api.DELETE("/api/v1/orgs/{org}/invitations/{id}", {
            params: { path: { org, id } },
          }),
        );
      }
      return null;
    }),
  );
}

export function Invitations() {
  const { invitations } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const invite = useFetcher();
  const rows = useFetcher();
  const { t, i18n } = useTranslation();
  const date = useDate();
  const sent =
    invite.data && typeof invite.data === "object" && "sent" in invite.data
      ? String(invite.data.sent)
      : null;
  const error = errorOf(invite.data) ?? errorOf(rows.data);
  return (
    <div className="grid gap-6">
      <Card>
        <h1 className="text-xl font-semibold">{t("invitations.title")}</h1>
        <invite.Form method="post" className="flex flex-wrap items-end gap-3">
          <input type="hidden" name="intent" value="invite" />
          <input type="hidden" name="language" value={i18n.language === "es" ? "es" : "en"} />
          <div className="grid min-w-64 flex-1 gap-2">
            <Label htmlFor="email">{t("invitations.email")}</Label>
            <Input id="email" name="email" type="email" required />
          </div>
          <Button type="submit">{t("invitations.invite")}</Button>
        </invite.Form>
        {sent && <p className="text-sm">{t("invitations.sent", { email: sent })}</p>}
        {error && (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        )}
      </Card>
      <Card>
        <table className="w-full text-left text-sm">
          <thead className="text-muted-foreground">
            <tr>
              <th className="py-2 pr-3">{t("invitations.email")}</th>
              <th className="py-2 pr-3">{t("invitations.state")}</th>
              <th className="py-2 pr-3">{t("invitations.inviter")}</th>
              <th className="py-2 pr-3">{t("invitations.expires")}</th>
              <th className="py-2" />
            </tr>
          </thead>
          <tbody className="divide-y">
            {invitations.map((invitation) => (
              <tr key={invitation.id}>
                <td className="py-2 pr-3">{invitation.email}</td>
                <td className="py-2 pr-3">
                  {t(`invitation.states.${invitation.state as "pending"}`)}
                </td>
                <td className="py-2 pr-3">{invitation.inviter}</td>
                <td className="py-2 pr-3">{date(invitation.expires_at)}</td>
                <td className="py-2">
                  {invitation.state === "pending" && (
                    <rows.Form method="post" className="flex justify-end gap-2">
                      <input type="hidden" name="id" value={invitation.id} />
                      <Button
                        type="submit"
                        name="intent"
                        value="resend"
                        size="sm"
                        variant="outline"
                      >
                        {t("invitations.resend")}
                      </Button>
                      <Button
                        type="submit"
                        name="intent"
                        value="revoke"
                        size="sm"
                        variant="destructive"
                      >
                        {t("invitations.revoke")}
                      </Button>
                    </rows.Form>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>
    </div>
  );
}
