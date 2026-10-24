import { Avatar, Icon, type IconName } from "@workspace/ui/taptap";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import type { Language } from "~/lib/language";
import { ACCOUNT_ENTRIES, ORGANIZATION_ACCOUNT_ENTRIES, organizationPath } from "./navigation";
import type { ShellOrganization, ShellPerson } from "./types";

export interface UserMenuProps {
  person: ShellPerson;
  organization?: ShellOrganization;
  language: string;
  onLanguage: (language: Language) => void;
  onSignOut: () => void;
  onNavigate?: () => void;
}

const LANGUAGES: Language[] = ["en", "es"];

const ITEM =
  "flex h-9 w-full cursor-pointer items-center gap-2.5 rounded-sm px-3 text-left text-sm text-[var(--neutral-700)] outline-none hover:bg-[var(--neutral-50)] focus-visible:bg-[var(--neutral-50)] focus-visible:shadow-[inset_0_0_0_2px_var(--primary-500)]";

const DIVIDER = "mt-1 border-t border-[var(--neutral-200)]";

export function UserMenu({
  person,
  organization,
  language,
  onLanguage,
  onSignOut,
  onNavigate,
}: UserMenuProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const items = () =>
    Array.from(
      menuRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"], [role="menuitemradio"]') ??
        [],
    );

  useEffect(() => {
    if (!open) {
      return;
    }
    const first = menuRef.current?.querySelector<HTMLElement>('[role="menuitem"]');
    first?.focus();
    const onPointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [open]);

  const close = (restoreFocus: boolean) => {
    setOpen(false);
    if (restoreFocus) {
      triggerRef.current?.focus();
    }
  };

  const onMenuKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const list = items();
    const index = list.indexOf(document.activeElement as HTMLElement);
    if (event.key === "Escape") {
      event.preventDefault();
      close(true);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      list[(index + 1) % list.length]?.focus();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      list[(index - 1 + list.length) % list.length]?.focus();
    } else if (event.key === "Home") {
      event.preventDefault();
      list[0]?.focus();
    } else if (event.key === "End") {
      event.preventDefault();
      list[list.length - 1]?.focus();
    } else if (event.key === "Tab") {
      close(false);
    }
  };

  const go = (path: string) => {
    close(false);
    navigate(path);
    onNavigate?.();
  };

  const entryItem = (id: string, icon: IconName, label: string, path: string) => (
    <button key={id} type="button" role="menuitem" className={ITEM} onClick={() => go(path)}>
      <Icon name={icon} size={16} />
      <span>{label}</span>
    </button>
  );

  return (
    <div ref={containerRef} className="relative flex-none border-t border-[var(--neutral-200)]">
      {open && (
        <div
          ref={menuRef}
          role="menu"
          aria-label={t("shell.userMenu")}
          onKeyDown={onMenuKeyDown}
          className="absolute inset-x-2 bottom-[64px] z-20 rounded bg-white p-1 shadow-[var(--elevation-2)]"
        >
          <div className="mb-1 flex flex-col border-b border-[var(--neutral-200)] px-3 pb-2 pt-2.5">
            <span className="text-sm font-medium leading-[22px]">{person.name}</span>
            <span className="text-xs leading-[18px] text-[var(--neutral-500)]">{person.email}</span>
            {person.isOperator && (
              <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                {t("shell.operator")}
              </span>
            )}
          </div>
          {ACCOUNT_ENTRIES.map((entry) =>
            entryItem(entry.id, entry.icon as IconName, t(entry.label), entry.path),
          )}
          {organization && (
            <>
              <div
                className={`${DIVIDER} flex items-center gap-1.5 px-3 pb-1 pt-2.5 text-xs text-[var(--neutral-500)]`}
              >
                <span className="uppercase">{t("shell.organization")}</span>
                <span className="font-medium text-[var(--neutral-700)]">{organization.name}</span>
              </div>
              {ORGANIZATION_ACCOUNT_ENTRIES.map((entry) =>
                entryItem(
                  entry.id,
                  entry.icon as IconName,
                  t(entry.label),
                  organizationPath(organization.slug, entry.section),
                ),
              )}
            </>
          )}
          <div className={`${DIVIDER} px-3 pb-1 pt-2.5 text-xs text-[var(--neutral-500)]`}>
            {t("app.language")}
          </div>
          {LANGUAGES.map((code) => (
            <button
              key={code}
              type="button"
              role="menuitemradio"
              aria-checked={language === code}
              className={ITEM}
              onClick={() => {
                onLanguage(code);
                close(false);
              }}
            >
              <span className="flex w-4 text-[var(--primary-600)]">
                {language === code && <Icon name="Check" size={16} />}
              </span>
              <span>{t(`app.languages.${code}`)}</span>
            </button>
          ))}
          <button
            type="button"
            role="menuitem"
            className={`${ITEM} ${DIVIDER} rounded-t-none text-[var(--danger-600)]`}
            onClick={() => {
              close(false);
              onSignOut();
            }}
          >
            <Icon name="Export" size={16} />
            <span>{t("app.signOut")}</span>
          </button>
        </div>
      )}
      <button
        ref={triggerRef}
        type="button"
        aria-label={t("shell.userMenu")}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
        className="flex w-full cursor-pointer items-center gap-2.5 px-5 py-4 text-left outline-none hover:bg-[var(--neutral-50)] focus-visible:shadow-[inset_0_0_0_2px_var(--primary-500)]"
      >
        <Avatar name={person.name} size={32} aria-hidden="true" />
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-sm font-medium leading-5">{person.name}</span>
          <span className="truncate font-mono text-xs leading-[18px] text-[var(--neutral-500)]">
            @{person.handle}
          </span>
        </span>
        <Icon name="Up" size={16} style={{ color: "var(--neutral-500)" }} />
      </button>
    </div>
  );
}
