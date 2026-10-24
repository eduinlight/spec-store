import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { useTranslation } from "react-i18next";
import {
  type ActionFunctionArgs,
  Form,
  type LoaderFunctionArgs,
  redirect,
  useActionData,
  useLoaderData,
} from "react-router";
import { api, succeeded, unwrap } from "~/api/client";
import { attempt, errorOf } from "~/lib/actions";
import { useDate } from "~/lib/format";
import { guard } from "~/lib/session";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const token = params.token ?? "";
  return guard(request, async () =>
    api.GET("/api/v1/invitations/{token}", { params: { path: { token } } }).then(unwrap),
  );
}

export async function action({ request, params }: ActionFunctionArgs) {
  const token = params.token ?? "";
  const form = await request.formData();
  const intent = String(form.get("intent"));
  return guard(request, () =>
    attempt(async () => {
      if (intent === "accept") {
        succeeded(
          await api.POST("/api/v1/invitations/{token}/accept", { params: { path: { token } } }),
        );
        const project = String(form.get("project") ?? "");
        return redirect(project ? `/p/${project}` : `/o/${String(form.get("organization"))}`);
      }
      succeeded(
        await api.POST("/api/v1/invitations/{token}/decline", { params: { path: { token } } }),
      );
      return { declined: true };
    }),
  );
}

export function InvitationAccept() {
  const invitation = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const result = useActionData();
  const { t } = useTranslation();
  const date = useDate();
  const declined = Boolean(result && typeof result === "object" && "declined" in result);
  const error = errorOf(result);
  const projectName = invitation.project ? (invitation.project_name ?? invitation.project) : null;
  return (
    <Card className="mx-auto max-w-lg">
      <h1 className="text-xl font-semibold">
        {projectName
          ? t("invitation.projectTitle", { project: projectName })
          : t("invitation.title", { organization: invitation.organization_name })}
      </h1>
      <p>
        {projectName
          ? t("invitation.projectBody", {
              project: projectName,
              role: t(`projects.roles.${(invitation.project_role ?? "reader") as "reader"}`),
            })
          : t("invitation.body", {
              inviter: invitation.inviter,
              email: invitation.email,
              organization: invitation.organization_name,
            })}
      </p>
      {projectName && (
        <p className="text-sm text-muted-foreground">
          {t("invitation.projectGuestNote", { organization: invitation.organization_name })}
        </p>
      )}
      {invitation.state === "pending" ? (
        <p className="text-sm text-muted-foreground">
          {t("invitation.expires", { when: date(invitation.expires_at) })}
        </p>
      ) : (
        <p className="text-sm">{t(`invitation.states.${invitation.state as "pending"}`)}</p>
      )}
      {declined ? (
        <p>{t("invitation.declined")}</p>
      ) : (
        invitation.state === "pending" && (
          <Form method="post" className="flex gap-2">
            <input type="hidden" name="organization" value={invitation.organization} />
            <input type="hidden" name="project" value={invitation.project ?? ""} />
            <Button type="submit" name="intent" value="accept">
              {t("invitation.accept")}
            </Button>
            <Button type="submit" name="intent" value="decline" variant="outline">
              {t("invitation.decline")}
            </Button>
          </Form>
        )
      )}
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </Card>
  );
}
