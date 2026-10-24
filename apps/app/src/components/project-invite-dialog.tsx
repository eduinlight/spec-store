import { Alert, Button, Dialog, Input, Select, type SelectValue } from "@workspace/ui/taptap";
import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { useFetcher } from "react-router";
import { errorOf } from "~/lib/actions";
import { useOutcomeToast } from "~/shell/outcome";

export const PROJECT_ROLES = ["reader", "editor", "owner"] as const;
export type ProjectRole = (typeof PROJECT_ROLES)[number];

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export const isEmail = (value: string) => EMAIL_PATTERN.test(value.trim());

export interface ProjectInviteDialogProps {
  projectName: string;
  organizationName: string;
  onClose: () => void;
}

export function ProjectInviteDialog({
  projectName,
  organizationName,
  onClose,
}: ProjectInviteDialogProps) {
  const { t, i18n } = useTranslation();
  const fetcher = useFetcher();
  const ids = { email: useId(), role: useId() };
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<ProjectRole>("reader");
  const sending = fetcher.state !== "idle";
  const invalid = email.trim() !== "" && !isEmail(email);
  const ready = isEmail(email) && !sending;
  const error = errorOf(fetcher.data);
  const done =
    fetcher.data && typeof fetcher.data === "object" && "done" in fetcher.data
      ? fetcher.data.done
      : null;

  useOutcomeToast(fetcher, (_done, data) =>
    typeof data.added === "string"
      ? t("toast.invitedAdded", {
          handle: data.added,
          role: t(`projects.roles.${role}`),
        })
      : t("toast.invitedEmail", { email: String(data.invited) }),
  );

  useEffect(() => {
    if (done === "invite") {
      onClose();
    }
  }, [done, onClose]);

  const submit = () => {
    if (!ready) {
      return;
    }
    fetcher.submit(
      {
        intent: "invite",
        email: email.trim(),
        role,
        language: i18n.language === "es" ? "es" : "en",
      },
      { method: "post" },
    );
  };

  return (
    <Dialog
      open
      title={t("projectMembers.dialog.title", { project: projectName })}
      size={480}
      onCancel={onClose}
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            {t("projectMembers.dialog.cancel")}
          </Button>
          <Button disabled={!ready} onClick={submit}>
            {sending ? t("projectMembers.dialog.sending") : t("projectMembers.dialog.send")}
          </Button>
        </>
      }
    >
      <form
        className="flex flex-col gap-4 text-[var(--neutral-700)]"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <div className="flex flex-col gap-1.5">
          <label htmlFor={ids.email} className="text-sm">
            {t("projectMembers.dialog.email")}
          </label>
          <Input
            id={ids.email}
            type="email"
            value={email}
            error={invalid}
            placeholder={t("projectMembers.dialog.emailPlaceholder")}
            onChange={setEmail}
          />
          {invalid && (
            <span className="text-xs leading-[18px] text-[var(--danger-600)]">
              {t("projectMembers.dialog.emailInvalid")}
            </span>
          )}
          <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
            {t("projectMembers.dialog.guestNote", { organization: organizationName })}
          </span>
        </div>
        <div className="flex flex-col gap-1.5">
          <label htmlFor={ids.role} className="text-sm">
            {t("projectMembers.dialog.role")}
          </label>
          <Select
            id={ids.role}
            aria-label={t("projectMembers.dialog.role")}
            width="100%"
            options={PROJECT_ROLES.map((value) => ({
              label: t(`projects.roles.${value}`),
              value,
            }))}
            value={role}
            onChange={(value: SelectValue) => setRole(value as ProjectRole)}
          />
          <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
            {t("projectMembers.dialog.roleNote")}
          </span>
        </div>
        {error && (
          <Alert status="error" closable={false} showIcon>
            {error}
          </Alert>
        )}
      </form>
    </Dialog>
  );
}
