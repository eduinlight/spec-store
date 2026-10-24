import { SidebarItem } from "@workspace/ui/patterns";
import { Icon, LockDocument } from "@workspace/ui/taptap";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { followLink } from "./links";
import {
  ACCOUNT_ENTRIES,
  accountMode,
  isSelected,
  type NavEntry,
  ORGANIZATION_ACCOUNT_ENTRIES,
  ORGANIZATION_ENTRIES,
  organizationPath,
  type Place,
  PROJECT_MAIN,
  PROJECT_SDD,
  PROJECT_SECTION,
  projectPath,
} from "./navigation";
import type { ShellOrganization } from "./types";

export interface BackTarget {
  path: string;
  label: string;
}

export interface SidebarProps {
  place: Place;
  organization?: ShellOrganization;
  back?: BackTarget;
  onNavigate?: () => void;
}

const ICON_SIZE = 18;

const renderIcon = (entry: NavEntry): ReactNode =>
  entry.icon === "LockDocument" ? (
    <LockDocument size={ICON_SIZE} />
  ) : (
    <Icon name={entry.icon} size={ICON_SIZE} />
  );

interface GroupLabelProps {
  children: ReactNode;
}

function GroupLabel({ children }: GroupLabelProps) {
  return (
    <div className="px-5 pb-1.5 pt-4 text-xs leading-[18px] text-[var(--neutral-500)]">
      {children}
    </div>
  );
}

interface EntryListProps {
  entries: NavEntry[];
  place: Place;
  pathOf: (entry: NavEntry) => string;
  onNavigate?: () => void;
}

function EntryList({ entries, place, pathOf, onNavigate }: EntryListProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  return (
    <div className="flex flex-col gap-0.5">
      {entries.map((entry) => {
        const path = pathOf(entry);
        return (
          <SidebarItem
            key={entry.id}
            label={t(entry.label)}
            href={path}
            icon={renderIcon(entry)}
            selected={isSelected(place, entry.section)}
            onClick={followLink(navigate, path, onNavigate)}
          />
        );
      })}
    </div>
  );
}

export function Sidebar({ place, organization, back, onNavigate }: SidebarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const org = organization?.slug ?? place.org ?? "";
  let content: ReactNode = null;
  if (accountMode(place)) {
    const backPath = back?.path ?? "/";
    content = (
      <>
        <a
          href={backPath}
          onClick={followLink(navigate, backPath, onNavigate)}
          className="flex h-[52px] items-center gap-2.5 border-b border-[var(--neutral-200)] px-5 text-sm text-[var(--neutral-600)] hover:bg-[var(--neutral-50)] hover:text-[var(--neutral-700)]"
        >
          <Icon name="ArrowLeft" size={ICON_SIZE} style={{ transform: "scaleX(-1)" }} />
          <span className="min-w-0 flex-1 truncate">
            {t("shell.backTo")}{" "}
            <span className="font-mono text-[var(--neutral-700)]">{back?.label ?? ""}</span>
          </span>
        </a>
        <GroupLabel>{t("shell.account")}</GroupLabel>
        <EntryList
          entries={ACCOUNT_ENTRIES}
          place={place}
          pathOf={(entry) => ACCOUNT_ENTRIES.find((item) => item.id === entry.id)?.path ?? "/"}
          onNavigate={onNavigate}
        />
        <div className="mx-5 mt-3 h-px bg-[var(--neutral-200)]" />
        {org && (
          <>
            <GroupLabel>
              {t("shell.organization")} ·{" "}
              <span className="font-medium text-[var(--neutral-700)]">
                {organization?.name ?? org}
              </span>
            </GroupLabel>
            <EntryList
              entries={ORGANIZATION_ACCOUNT_ENTRIES}
              place={place}
              pathOf={(entry) => organizationPath(org, entry.section)}
              onNavigate={onNavigate}
            />
          </>
        )}
      </>
    );
  } else if (place.area === "project" && place.project) {
    const project = place.project;
    const pathOf = (entry: NavEntry) => projectPath(place.org ?? org, project, entry.section);
    content = (
      <div className="py-3">
        <EntryList entries={PROJECT_MAIN} place={place} pathOf={pathOf} onNavigate={onNavigate} />
        <GroupLabel>{t("shell.sdd")}</GroupLabel>
        <EntryList entries={PROJECT_SDD} place={place} pathOf={pathOf} onNavigate={onNavigate} />
        <GroupLabel>{t("shell.project")}</GroupLabel>
        <EntryList
          entries={PROJECT_SECTION}
          place={place}
          pathOf={pathOf}
          onNavigate={onNavigate}
        />
      </div>
    );
  } else if (place.area === "organization" && org) {
    const entries =
      organization?.role === "owner"
        ? ORGANIZATION_ENTRIES
        : ORGANIZATION_ENTRIES.filter((entry) => entry.id !== "org-invitations");
    content = (
      <div className="py-3">
        <EntryList
          entries={entries}
          place={place}
          pathOf={(entry) => organizationPath(org, entry.section)}
          onNavigate={onNavigate}
        />
      </div>
    );
  }
  return (
    <nav
      aria-label={t("shell.mainNavigation")}
      className="flex min-h-0 flex-1 flex-col overflow-y-auto"
    >
      {content}
    </nav>
  );
}
