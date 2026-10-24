import { SidebarItem } from "@workspace/ui/patterns";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { followLink } from "./links";
import type { ShellOrganization } from "./types";

export interface OrganizationListProps {
  organizations: ShellOrganization[];
  current?: string;
  pathOf: (slug: string) => string;
  onPick: (slug: string) => void;
}

export function OrganizationList({
  organizations,
  current,
  pathOf,
  onPick,
}: OrganizationListProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  if (organizations.length === 0) {
    return null;
  }
  return (
    // biome-ignore lint/a11y/useSemanticElements: a labelled group of links, a fieldset is a form control container
    <div
      role="group"
      aria-label={t("shell.organizations")}
      className="border-b border-[var(--neutral-200)] py-2"
    >
      <div className="px-5 pb-1.5 pt-2 text-xs leading-[18px] text-[var(--neutral-500)]">
        {t("shell.organizations")}
      </div>
      {organizations.map((organization) => {
        const path = pathOf(organization.slug);
        return (
          <SidebarItem
            key={organization.slug}
            label={organization.name}
            href={path}
            selected={organization.slug === current}
            onClick={followLink(navigate, path, () => onPick(organization.slug))}
          />
        );
      })}
    </div>
  );
}
