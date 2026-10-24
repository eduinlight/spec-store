import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { useTranslation } from "react-i18next";
import {
  type ActionFunctionArgs,
  type LoaderFunctionArgs,
  redirect,
  useFetcher,
  useLoaderData,
} from "react-router";
import { api, succeeded, unwrap } from "~/api/client";
import { attempt, errorOf } from "~/lib/actions";
import { guard } from "~/lib/session";
import { useConfirm } from "~/shell/confirm";
import { useOutcomeToast } from "~/shell/outcome";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const org = params.org ?? "";
  return guard(request, async () => {
    const [organization, members, me] = await Promise.all([
      api.GET("/api/v1/orgs/{org}", { params: { path: { org } } }).then(unwrap),
      api.GET("/api/v1/orgs/{org}/members", { params: { path: { org } } }).then(unwrap),
      api.GET("/api/v1/me").then(unwrap),
    ]);
    return { organization, members: members.members, me };
  });
}

export async function action({ request, params }: ActionFunctionArgs) {
  const org = params.org ?? "";
  const form = await request.formData();
  const handle = String(form.get("handle"));
  const intent = String(form.get("intent"));
  return guard(request, () =>
    attempt(async () => {
      if (intent === "remove" || intent === "leave") {
        succeeded(
          await api.DELETE("/api/v1/orgs/{org}/members/{handle}", {
            params: { path: { org, handle } },
          }),
        );
        return intent === "leave" ? redirect("/") : { done: intent };
      }
      const role = String(form.get("role"));
      succeeded(
        await api.PUT("/api/v1/orgs/{org}/members/{handle}", {
          params: { path: { org, handle } },
          body: { role },
        }),
      );
      return { done: intent };
    }),
  );
}

export function Members() {
  const { organization, members, me } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const fetcher = useFetcher();
  const { t } = useTranslation();
  const owner = organization.role === "owner";
  const error = errorOf(fetcher.data);
  const confirm = useConfirm();
  useOutcomeToast(fetcher, (intent) =>
    intent === "role"
      ? t("toast.roleChanged")
      : intent === "remove"
        ? t("toast.memberRemoved")
        : undefined,
  );
  const ask = async (intent: "remove" | "leave", handle: string) => {
    const confirmed = await confirm(
      intent === "remove"
        ? {
            title: t("confirm.removeMemberTitle", { handle }),
            body: t("confirm.removeMemberBody", { handle }),
            confirmText: t("members.remove"),
          }
        : {
            title: t("confirm.leaveTitle", { organization: organization.name }),
            body: t("confirm.leaveBody"),
            confirmText: t("members.leave"),
          },
    );
    if (confirmed) {
      fetcher.submit({ handle, intent }, { method: "post" });
    }
  };
  return (
    <Card>
      <h1 className="text-xl font-semibold">
        {t("members.title", { organization: organization.name })}
      </h1>
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
      <table className="w-full text-left text-sm">
        <thead className="text-muted-foreground">
          <tr>
            <th className="py-2 pr-3">{t("members.handle")}</th>
            <th className="py-2 pr-3">{t("members.role")}</th>
            <th className="py-2" />
          </tr>
        </thead>
        <tbody className="divide-y">
          {members.map((member) => (
            <tr key={member.handle}>
              <td className="py-2 pr-3">
                {member.handle} <span className="text-muted-foreground">{member.name}</span>
              </td>
              <td className="py-2 pr-3">{t(`organization.roles.${member.role as "owner"}`)}</td>
              <td className="py-2">
                <fetcher.Form method="post" className="flex justify-end gap-2">
                  <input type="hidden" name="handle" value={member.handle} />
                  {owner && (
                    <>
                      <input
                        type="hidden"
                        name="role"
                        value={member.role === "owner" ? "member" : "owner"}
                      />
                      <Button type="submit" name="intent" value="role" size="sm" variant="outline">
                        {member.role === "owner" ? t("members.makeMember") : t("members.makeOwner")}
                      </Button>
                    </>
                  )}
                  {owner && member.handle !== me.handle && (
                    <Button
                      type="button"
                      size="sm"
                      variant="destructive"
                      onClick={() => void ask("remove", member.handle)}
                    >
                      {t("members.remove")}
                    </Button>
                  )}
                  {member.handle === me.handle && (
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      onClick={() => void ask("leave", member.handle)}
                    >
                      {t("members.leave")}
                    </Button>
                  )}
                </fetcher.Form>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Card>
  );
}
