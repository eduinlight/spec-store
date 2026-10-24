import { Button } from "@workspace/ui/components/button";
import { Card } from "@workspace/ui/components/card";
import { Input } from "@workspace/ui/components/input";
import { Label } from "@workspace/ui/components/label";
import { useTranslation } from "react-i18next";
import {
  type ActionFunctionArgs,
  Link,
  type LoaderFunctionArgs,
  useFetcher,
  useLoaderData,
} from "react-router";
import { api, succeeded, unwrap } from "~/api/client";
import { attempt, errorOf } from "~/lib/actions";
import { guard } from "~/lib/session";
import { useConfirm } from "~/shell/confirm";
import { useOutcomeToast } from "~/shell/outcome";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  return guard(request, async () => {
    const [project, members, people] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      api.GET("/api/v1/projects/{org}/{project}/members", { params: { path } }).then(unwrap),
      api.GET("/api/v1/orgs/{org}/members", { params: { path: { org: path.org } } }).then(unwrap),
    ]);
    return { project, members: members.members, people: people.members };
  });
}

export async function action({ request, params }: ActionFunctionArgs) {
  const path = projectParams(params);
  const form = await request.formData();
  const intent = String(form.get("intent"));
  return guard(request, () =>
    attempt(async () => {
      if (intent === "grant") {
        const handle = String(form.get("handle"));
        const role = String(form.get("role"));
        succeeded(
          await api.PUT("/api/v1/projects/{org}/{project}/members/{handle}", {
            params: { path: { ...path, handle } },
            body: { role },
          }),
        );
      } else if (intent === "remove") {
        const handle = String(form.get("handle"));
        succeeded(
          await api.DELETE("/api/v1/projects/{org}/{project}/members/{handle}", {
            params: { path: { ...path, handle } },
          }),
        );
      } else if (intent === "archive" || intent === "restore") {
        succeeded(
          await api.POST(`/api/v1/projects/{org}/{project}/${intent}`, { params: { path } }),
        );
        return { done: intent };
      } else if (intent === "repository") {
        const url = String(form.get("url") ?? "").trim();
        succeeded(
          await api.PUT("/api/v1/projects/{org}/{project}/repository-url", {
            params: { path },
            body: { url },
          }),
        );
      } else {
        succeeded(
          await api.DELETE("/api/v1/projects/{org}/{project}/repository-url", { params: { path } }),
        );
      }
      return null;
    }),
  );
}

export function ProjectSettings() {
  const { project, members, people } = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const roles = useFetcher();
  const repository = useFetcher();
  const lifecycle = useFetcher();
  const { t } = useTranslation();
  const confirm = useConfirm();
  const error = errorOf(roles.data) ?? errorOf(repository.data) ?? errorOf(lifecycle.data);
  useOutcomeToast(lifecycle, (done) =>
    done === "archive" ? t("toast.projectArchived") : t("toast.projectRestored"),
  );
  const toggleArchive = async () => {
    const intent = project.archived ? "restore" : "archive";
    const confirmed = await confirm(
      project.archived
        ? {
            title: t("archive.restoreTitle", { name: project.name }),
            body: t("archive.restoreBody"),
            confirmText: t("archive.restore"),
          }
        : {
            title: t("archive.confirmTitle", { name: project.name }),
            body: t("archive.confirmBody"),
            confirmText: t("archive.archive"),
          },
    );
    if (confirmed) {
      lifecycle.submit({ intent }, { method: "post" });
    }
  };
  return (
    <div className="grid gap-6">
      <div>
        <Link
          to={`/p/${project.address}`}
          className="text-sm text-muted-foreground hover:underline"
        >
          {project.name}
        </Link>
        <h1 className="text-2xl font-semibold">{t("projects.settings")}</h1>
      </div>
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
      <Card>
        <h2 className="text-lg font-semibold">{t("projects.members")}</h2>
        <roles.Form method="post" className="flex flex-wrap items-end gap-3">
          <input type="hidden" name="intent" value="grant" />
          <div className="grid gap-2">
            <Label htmlFor="handle">{t("members.handle")}</Label>
            <select id="handle" name="handle" className="h-9 rounded-md border bg-background px-2">
              {people.map((person) => (
                <option key={person.handle} value={person.handle}>
                  {person.handle}
                </option>
              ))}
            </select>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="role">{t("members.role")}</Label>
            <select id="role" name="role" className="h-9 rounded-md border bg-background px-2">
              <option value="reader">{t("projects.roles.reader")}</option>
              <option value="editor">{t("projects.roles.editor")}</option>
              <option value="owner">{t("projects.roles.owner")}</option>
            </select>
          </div>
          <Button type="submit">{t("projects.grant")}</Button>
        </roles.Form>
        <ul className="divide-y text-sm">
          {members.map((member) => (
            <li key={member.handle} className="flex items-center justify-between py-2">
              <span>
                {member.handle}{" "}
                <span className="text-muted-foreground">
                  {t(`projects.roles.${member.role as "owner"}`)}
                </span>
              </span>
              <roles.Form method="post">
                <input type="hidden" name="intent" value="remove" />
                <input type="hidden" name="handle" value={member.handle} />
                <Button type="submit" size="sm" variant="outline">
                  {t("app.remove")}
                </Button>
              </roles.Form>
            </li>
          ))}
        </ul>
      </Card>
      <Card>
        <h2 className="text-lg font-semibold">{t("projects.repository")}</h2>
        <repository.Form method="post" className="flex flex-wrap items-end gap-3">
          <div className="grid min-w-72 flex-1 gap-2">
            <Label htmlFor="url">{t("projects.repository")}</Label>
            <Input id="url" name="url" type="url" defaultValue={project.repository_url ?? ""} />
          </div>
          <Button type="submit" name="intent" value="repository">
            {t("app.save")}
          </Button>
          <Button type="submit" name="intent" value="clear" variant="outline">
            {t("projects.clearRepository")}
          </Button>
        </repository.Form>
      </Card>
      <Card>
        <h2 className="text-lg font-semibold">{t("archive.title")}</h2>
        <p className="text-sm text-muted-foreground">
          {project.archived ? t("archive.restoreDescription") : t("archive.archiveDescription")}
        </p>
        <div>
          <Button
            type="button"
            variant={project.archived ? "outline" : "destructive"}
            onClick={() => void toggleArchive()}
          >
            {project.archived ? t("archive.restore") : t("archive.archive")}
          </Button>
        </div>
      </Card>
    </div>
  );
}
