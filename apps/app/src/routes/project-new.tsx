import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import { useTranslation } from "react-i18next";
import { type ActionFunctionArgs, Form, redirect, useActionData } from "react-router";
import { api, unwrap } from "~/api/client";
import { attempt, errorOf } from "~/lib/actions";
import { guard } from "~/lib/session";

export async function action({ request, params }: ActionFunctionArgs) {
  const org = params.org ?? "";
  const form = await request.formData();
  const slug = String(form.get("slug") ?? "").trim();
  const name = String(form.get("name") ?? "").trim();
  return guard(request, () =>
    attempt(async () => {
      unwrap(
        await api.POST("/api/v1/orgs/{org}/projects", {
          params: { path: { org } },
          body: { slug, name },
        }),
      );
      return redirect(`/p/${org}/${slug}/settings`);
    }),
  );
}

export function ProjectNew() {
  const result = useActionData();
  const { t } = useTranslation();
  const error = errorOf(result);
  return (
    <Card className="mx-auto max-w-lg">
      <h1 className="text-xl font-semibold">{t("projects.new")}</h1>
      <Form method="post" className="grid gap-4">
        <div className="grid gap-2">
          <Label htmlFor="slug">{t("projects.slug")}</Label>
          <Input id="slug" name="slug" required pattern="[a-z0-9]+(-[a-z0-9]+){0,3}" />
          <p className="text-xs text-muted-foreground">{t("onboarding.slugHint")}</p>
        </div>
        <div className="grid gap-2">
          <Label htmlFor="name">{t("projects.name")}</Label>
          <Input id="name" name="name" required />
        </div>
        {error && (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        )}
        <Button type="submit">{t("projects.create")}</Button>
      </Form>
    </Card>
  );
}
