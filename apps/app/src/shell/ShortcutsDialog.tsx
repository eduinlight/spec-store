import { Kbd } from "@workspace/ui/patterns";
import { Dialog, ModalButton } from "@workspace/ui/taptap";
import type { ParseKeys } from "i18next";
import { useTranslation } from "react-i18next";
import { paletteKeys } from "./platform";

export interface ShortcutsDialogProps {
  open: boolean;
  onClose: () => void;
}

interface ShortcutRow {
  label: ParseKeys;
  keys: string[];
}

interface ShortcutGroup {
  label: ParseKeys;
  rows: ShortcutRow[];
}

const chord = (key: string): string[] => ["g", key];

const groups = (): ShortcutGroup[] => [
  {
    label: "shell.shortcutsDialog.general",
    rows: [
      { label: "shell.shortcutsDialog.palette", keys: paletteKeys() },
      { label: "shell.shortcutsDialog.notifications", keys: ["n"] },
      { label: "shell.shortcutsDialog.shortcuts", keys: ["?"] },
      { label: "shell.shortcutsDialog.close", keys: ["esc"] },
    ],
  },
  {
    label: "shell.shortcutsDialog.goTo",
    rows: [
      { label: "shell.shortcutsDialog.overview", keys: chord("o") },
      { label: "shell.shortcutsDialog.active", keys: chord("e") },
      { label: "shell.shortcutsDialog.specs", keys: chord("l") },
      { label: "shell.shortcutsDialog.constitution", keys: chord("c") },
    ],
  },
  {
    label: "shell.shortcutsDialog.project",
    rows: [
      { label: "shell.shortcutsDialog.activity", keys: chord("a") },
      { label: "shell.shortcutsDialog.members", keys: chord("m") },
      { label: "shell.shortcutsDialog.audit", keys: chord("u") },
      { label: "shell.shortcutsDialog.profile", keys: chord("p") },
    ],
  },
];

export function ShortcutsDialog({ open, onClose }: ShortcutsDialogProps) {
  const { t } = useTranslation();
  return (
    <Dialog
      open={open}
      title={t("shell.shortcuts")}
      size={560}
      onCancel={onClose}
      footer={<ModalButton onClick={onClose}>{t("shell.shortcutsDialog.done")}</ModalButton>}
    >
      <div className="grid gap-x-8 gap-y-5 [grid-template-columns:repeat(auto-fit,minmax(220px,1fr))]">
        {groups().map((group) => (
          <div key={group.label} className="flex flex-col gap-0.5">
            <span className="mb-1.5 text-xs font-medium leading-[18px] text-[var(--neutral-500)]">
              {t(group.label)}
            </span>
            {group.rows.map((row) => (
              <div
                key={row.label}
                className="flex h-8 items-center gap-2 text-sm text-[var(--neutral-700)]"
              >
                <span className="flex-1">{t(row.label)}</span>
                <span className="flex gap-1">
                  {row.keys.map((key) => (
                    <Kbd key={key}>{key}</Kbd>
                  ))}
                </span>
              </div>
            ))}
          </div>
        ))}
      </div>
    </Dialog>
  );
}
