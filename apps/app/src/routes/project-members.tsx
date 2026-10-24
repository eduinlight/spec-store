import { cn } from "@workspace/ui/lib/utils";
import {
  Avatar,
  Button,
  Icon,
  IconButton,
  message,
  Select,
  Tag,
  Tooltip,
} from "@workspace/ui/taptap";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  type ActionFunctionArgs,
  type LoaderFunctionArgs,
  useFetcher,
  useLoaderData,
} from "react-router";
import { api, succeeded, unwrap } from "~/api/client";
import { PROJECT_ROLES, ProjectInviteDialog } from "~/components/project-invite-dialog";
import { attempt, errorOf } from "~/lib/actions";
import { useDate } from "~/lib/format";
import { guard } from "~/lib/session";
import { useConfirm } from "~/shell/confirm";
import { useOutcomeToast } from "~/shell/outcome";
import { projectParams } from "./project";

export async function loader({ request, params }: LoaderFunctionArgs) {
  const path = projectParams(params);
  return guard(request, async () => {
    const [project, organization, members, me] = await Promise.all([
      api.GET("/api/v1/projects/{org}/{project}", { params: { path } }).then(unwrap),
      api.GET("/api/v1/orgs/{org}", { params: { path: { org: path.org } } }).then(unwrap),
      api.GET("/api/v1/projects/{org}/{project}/members", { params: { path } }).then(unwrap),
      api.GET("/api/v1/me").then(unwrap),
    ]);
    const invitations =
      project.role === "owner"
        ? await api
            .GET("/api/v1/projects/{org}/{project}/members/invitations", { params: { path } })
            .then(unwrap)
        : { invitations: [] };
    return {
      project,
      organization,
      me,
      members: members.members,
      invitations: invitations.invitations.filter((item) => item.state === "pending"),
    };
  });
}

export async function action({ request, params }: ActionFunctionArgs) {
  const path = projectParams(params);
  const form = await request.formData();
  const intent = String(form.get("intent"));
  const handle = String(form.get("handle") ?? "");
  const id = Number(form.get("id"));
  return guard(request, () =>
    attempt(async () => {
      if (intent === "role") {
        succeeded(
          await api.PUT("/api/v1/projects/{org}/{project}/members/{handle}", {
            params: { path: { ...path, handle } },
            body: { role: String(form.get("role")) },
          }),
        );
      } else if (intent === "remove") {
        succeeded(
          await api.DELETE("/api/v1/projects/{org}/{project}/members/{handle}", {
            params: { path: { ...path, handle } },
          }),
        );
      } else if (intent === "resend") {
        succeeded(
          await api.POST("/api/v1/projects/{org}/{project}/members/invitations/{id}/resend", {
            params: { path: { ...path, id } },
          }),
        );
      } else if (intent === "revoke") {
        succeeded(
          await api.DELETE("/api/v1/projects/{org}/{project}/members/invitations/{id}", {
            params: { path: { ...path, id } },
          }),
        );
      } else {
        const result = await api.POST("/api/v1/projects/{org}/{project}/members/invitations", {
          params: { path },
          body: {
            email: String(form.get("email")),
            role: String(form.get("role")),
            language: String(form.get("language") ?? "en"),
          },
        });
        succeeded(result);
        return { done: intent, added: result.data?.added, invited: result.data?.invited };
      }
      return { done: intent };
    }),
  );
}

interface HeadCellProps {
  children?: ReactNode;
  className?: string;
}

const HeadCell = ({ children, className }: HeadCellProps) => (
  <th
    scope="col"
    className={cn(
      "px-2 py-[7px] text-left text-xs font-medium leading-[22px] text-[var(--neutral-600)]",
      className,
    )}
  >
    {children}
  </th>
);

const Cell = ({ children, className }: HeadCellProps) => (
  <td className={cn("px-2 py-2 text-[13px] leading-[22px] text-[var(--neutral-600)]", className)}>
    {children}
  </td>
);

export function ProjectMembers() {
  const { project, organization, me, members, invitations } = useLoaderData() as Awaited<
    ReturnType<typeof loader>
  >;
  const { t } = useTranslation();
  const date = useDate();
  const confirm = useConfirm();
  const rows = useFetcher();
  const [inviting, setInviting] = useState(false);
  const owner = project.role === "owner";
  const owners = members.filter(
    (member) => member.role === "owner" && !member.from_organization,
  ).length;
  const error = errorOf(rows.data);

  useOutcomeToast(rows, (done) =>
    done === "role"
      ? t("toast.roleChanged")
      : done === "remove"
        ? t("toast.memberRemoved")
        : done === "resend"
          ? t("toast.invitationResent")
          : done === "revoke"
            ? t("toast.invitationRevoked")
            : undefined,
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: a toast per refused action, keyed on the action result object
  useEffect(() => {
    if (error) {
      message.error(error);
    }
  }, [rows.data]);

  const changeRole = (handle: string, role: string) =>
    rows.submit({ intent: "role", handle, role }, { method: "post" });

  const remove = async (handle: string, name: string) => {
    const confirmed = await confirm({
      title: t("confirm.removeProjectMemberTitle", { name }),
      body: t("confirm.removeProjectMemberBody", { name, project: project.name }),
      confirmText: t("projectMembers.removeAction"),
    });
    if (confirmed) {
      rows.submit({ intent: "remove", handle }, { method: "post" });
    }
  };

  const revoke = async (id: number, email: string) => {
    const confirmed = await confirm({
      title: t("confirm.revokeInvitationTitle"),
      body: t("confirm.revokeInvitationBody", { email }),
      confirmText: t("projectMembers.revoke"),
    });
    if (confirmed) {
      rows.submit({ intent: "revoke", id: String(id) }, { method: "post" });
    }
  };

  return (
    <div className="mx-auto flex max-w-[1280px] flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3 pt-2">
        <h1 className="m-0 flex-1 text-2xl font-medium leading-8">{t("projectMembers.title")}</h1>
        {owner && (
          <Button leftIcon={<Icon name="Add" size={16} />} onClick={() => setInviting(true)}>
            {t("projectMembers.invite")}
          </Button>
        )}
      </div>
      <div className="overflow-x-auto rounded bg-white">
        <table aria-label={t("projectMembers.list")} className="w-full border-collapse">
          <thead>
            <tr className="bg-[var(--neutral-50)]">
              <HeadCell>{t("projectMembers.member")}</HeadCell>
              <HeadCell>{t("projectMembers.role")}</HeadCell>
              <HeadCell>{t("projectMembers.joined")}</HeadCell>
              <HeadCell>{t("projectMembers.lastActive")}</HeadCell>
              {owner && <HeadCell className="w-12" />}
            </tr>
          </thead>
          <tbody>
            {members.map((member) => {
              const locked =
                member.from_organization ||
                (member.role === "owner" && owners === 1 && member.handle === me.handle);
              return (
                <tr key={member.handle} className="border-b border-[var(--neutral-200)]">
                  <Cell>
                    <div className="flex items-center gap-3">
                      <Avatar name={member.name} size={32} />
                      <div className="flex min-w-0 flex-col">
                        <span className="text-sm font-medium text-[var(--neutral-800)]">
                          {member.name}
                        </span>
                        <span className="flex items-center gap-2 text-xs text-[var(--neutral-500)]">
                          <span>@{member.handle}</span>
                          {member.guest && <Tag type="basic">{t("projectMembers.guest")}</Tag>}
                          {member.from_organization && (
                            <Tag type="primary">{t("projectMembers.organizationOwner")}</Tag>
                          )}
                        </span>
                      </div>
                    </div>
                  </Cell>
                  <Cell>
                    {owner && !locked ? (
                      <Select
                        size="small"
                        width={130}
                        aria-label={t("projectMembers.roleOf", { name: member.name })}
                        options={PROJECT_ROLES.map((value) => ({
                          label: t(`projects.roles.${value}`),
                          value,
                        }))}
                        value={member.role}
                        onChange={(value) => changeRole(member.handle, String(value))}
                      />
                    ) : (
                      t(`projects.roles.${member.role as "owner"}`)
                    )}
                  </Cell>
                  <Cell>{date(member.joined_at)}</Cell>
                  <Cell>
                    {member.last_active_at
                      ? date(member.last_active_at)
                      : t("projectMembers.never")}
                  </Cell>
                  {owner && (
                    <Cell>
                      {!locked && (
                        <Tooltip title={t("projectMembers.remove", { name: member.name })}>
                          <IconButton
                            variant="ghost"
                            size="small"
                            icon={<Icon name="Delete" size={16} />}
                            aria-label={t("projectMembers.remove", { name: member.name })}
                            onClick={() => void remove(member.handle, member.name)}
                          />
                        </Tooltip>
                      )}
                    </Cell>
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      {owner && invitations.length > 0 && (
        <section aria-labelledby="pending-invitations" className="flex flex-col gap-2">
          <h2 id="pending-invitations" className="m-0 text-base font-medium leading-6">
            {t("projectMembers.pending")}
          </h2>
          <div className="overflow-x-auto rounded bg-white">
            <table className="w-full border-collapse">
              <thead>
                <tr className="bg-[var(--neutral-50)]">
                  <HeadCell>{t("projectMembers.email")}</HeadCell>
                  <HeadCell>{t("projectMembers.role")}</HeadCell>
                  <HeadCell>{t("projectMembers.expires")}</HeadCell>
                  <HeadCell />
                </tr>
              </thead>
              <tbody>
                {invitations.map((invitation) => (
                  <tr key={invitation.id} className="border-b border-[var(--neutral-200)]">
                    <Cell>{invitation.email}</Cell>
                    <Cell>{t(`projects.roles.${invitation.role as "owner"}`)}</Cell>
                    <Cell>{date(invitation.expires_at)}</Cell>
                    <Cell>
                      <div className="flex justify-end gap-2">
                        <Button
                          variant="outline"
                          size="small"
                          onClick={() =>
                            rows.submit(
                              { intent: "resend", id: String(invitation.id) },
                              { method: "post" },
                            )
                          }
                        >
                          {t("projectMembers.resend")}
                        </Button>
                        <Button
                          variant="outline"
                          danger
                          size="small"
                          onClick={() => void revoke(invitation.id, invitation.email)}
                        >
                          {t("projectMembers.revoke")}
                        </Button>
                      </div>
                    </Cell>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      )}
      {inviting && (
        <ProjectInviteDialog
          projectName={project.name}
          organizationName={organization.name}
          onClose={() => setInviting(false)}
        />
      )}
    </div>
  );
}
