import { Button } from "@workspace/ui/components/button";
import { Card, CardTitle } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import { useTranslation } from "react-i18next";
import {
  type ActionFunctionArgs,
  Form,
  type LoaderFunctionArgs,
  redirect,
  useActionData,
  useFetcher,
  useLoaderData,
} from "react-router";
import { ApiFailure, api, succeeded, unwrap } from "~/api/client";
import { guard } from "~/lib/session";

export async function loader({ request }: LoaderFunctionArgs) {
  return guard(request, async () => unwrap(await api.GET("/api/v1/invitations")).invitations);
}

export async function action({ request }: ActionFunctionArgs) {
  const form = await request.formData();
  const intent = String(form.get("intent") ?? "create");
  return guard(request, async () => {
    try {
      if (intent === "accept" || intent === "decline") {
        const id = Number(form.get("id"));
        const path =
          intent === "accept"
            ? "/api/v1/me/invitations/{id}/accept"
            : "/api/v1/me/invitations/{id}/decline";
        succeeded(await api.POST(path, { params: { path: { id } } }));
        const organization = String(form.get("organization"));
        return intent === "accept" ? redirect(`/o/${organization}`) : null;
      }
      const slug = String(form.get("slug") ?? "").trim();
      const name = String(form.get("name") ?? "").trim();
      unwrap(await api.POST("/api/v1/orgs", { body: { slug, name } }));
      return redirect(`/o/${slug}`);
    } catch (error) {
      if (error instanceof ApiFailure && error.status !== 401) return { error: error.message };
      throw error;
    }
  });
}

export function Onboarding() {
  const invitations = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const result = useActionData() as { error?: string } | null;
  const fetcher = useFetcher();
  const { t } = useTranslation();
  return (
    <div className="mx-auto grid max-w-2xl gap-6">
      <div>
        <h1 className="text-2xl font-semibold">{t("onboarding.title")}</h1>
        <p className="mt-2 text-muted-foreground">{t("onboarding.intro")}</p>
      </div>
      <Card>
        <Form method="post" className="grid gap-4">
          <input type="hidden" name="intent" value="create" />
          <div className="grid gap-2">
            <Label htmlFor="slug">{t("onboarding.slug")}</Label>
            <Input id="slug" name="slug" required pattern="[a-z0-9]+(-[a-z0-9]+){0,3}" />
            <p className="text-xs text-muted-foreground">{t("onboarding.slugHint")}</p>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="name">{t("onboarding.name")}</Label>
            <Input id="name" name="name" required />
          </div>
          {result?.error && (
            <p role="alert" className="text-sm text-destructive">
              {result.error}
            </p>
          )}
          <Button type="submit">{t("onboarding.create")}</Button>
        </Form>
      </Card>
      <Card>
        <CardTitle>{t("onboarding.pending")}</CardTitle>
        {invitations.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("onboarding.noPending")}</p>
        ) : (
          <ul className="grid gap-3">
            {invitations.map((invitation) => (
              <li key={invitation.id} className="flex items-center justify-between gap-4">
                <div>
                  <p className="font-medium">{invitation.organization_name}</p>
                  <p className="text-sm text-muted-foreground">
                    {t("onboarding.invitedBy", { inviter: invitation.inviter })}
                  </p>
                </div>
                <fetcher.Form method="post" className="flex gap-2">
                  <input type="hidden" name="id" value={invitation.id} />
                  <input type="hidden" name="organization" value={invitation.organization} />
                  <Button type="submit" name="intent" value="accept" size="sm">
                    {t("onboarding.accept")}
                  </Button>
                  <Button type="submit" name="intent" value="decline" size="sm" variant="outline">
                    {t("onboarding.decline")}
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
