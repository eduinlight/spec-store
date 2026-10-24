import { Combobox, type ComboboxItem, Kbd } from "@workspace/ui/patterns";
import { Icon, IconButton, Tooltip } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import { Link, useLocation, useNavigate } from "react-router";
import { organizationPath, switchOrganizationPath, switchProjectPath } from "./navigation";
import { paletteKeys } from "./platform";
import type { ShellOrganization, ShellProject } from "./types";

export interface HeaderProps {
  organizations: ShellOrganization[];
  projects: ShellProject[];
  org?: string;
  project?: string;
  activeSpec?: string;
  unread?: number;
  onOpenPalette: () => void;
  onOpenShortcuts: () => void;
  onOpenNotifications: () => void;
  onOpenMenu: () => void;
  onOrganizationChange?: (slug: string) => void;
}

function MenuIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none" aria-hidden="true">
      <path
        d="M3 5h14M3 10h14M3 15h14"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function Header({
  organizations,
  projects,
  org,
  project,
  activeSpec,
  unread = 0,
  onOpenPalette,
  onOpenShortcuts,
  onOpenNotifications,
  onOpenMenu,
  onOrganizationChange,
}: HeaderProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const currentOrganization = organizations.find((item) => item.slug === org);
  const organizationItems: ComboboxItem[] = organizations.map((item) => ({
    value: item.slug,
    label: item.name,
    meta: item.role ? t(`organization.roles.${item.role as "owner"}`) : t("app.operator"),
  }));
  const projectItems: ComboboxItem[] = projects.map((item) => ({
    value: item.slug,
    label: item.slug,
    description: item.slug === project && activeSpec ? activeSpec : item.name,
    meta: item.role ? t(`projects.roles.${item.role as "owner"}`) : t("app.operator"),
  }));
  const keys = paletteKeys();
  return (
    <header className="relative z-10 flex h-[60px] flex-none items-center gap-2 bg-[var(--primary-600)] px-3 text-white shadow-[0_1px_5px_rgba(45,47,51,.1)] sm:gap-4 sm:px-5">
      <div className="flex min-[880px]:hidden">
        <IconButton icon={<MenuIcon />} aria-label={t("shell.openMenu")} onClick={onOpenMenu} />
      </div>
      <Link to="/" className="hidden items-center sm:flex min-[880px]:w-[196px]">
        <img
          src="/logo-horizontal-white.png"
          alt={t("app.name")}
          height={26}
          className="block h-[26px] w-auto"
        />
      </Link>
      {currentOrganization && (
        <div className="flex min-w-0 items-center gap-1">
          <div className="hidden sm:block">
            <Combobox
              items={organizationItems}
              value={org}
              groupLabel={t("shell.organizations")}
              searchPlaceholder={t("shell.searchOrganization")}
              emptyText={t("shell.noOrganization")}
              aria-label={t("shell.organization")}
              onChange={(slug) => {
                onOrganizationChange?.(slug);
                navigate(switchOrganizationPath(pathname, slug));
              }}
              actions={[
                {
                  key: "create-organization",
                  label: t("shell.createOrganization"),
                  icon: "PlusSmall",
                  onSelect: () => navigate("/onboarding"),
                },
              ]}
            />
          </div>
          <span
            aria-hidden="true"
            className="hidden px-0.5 text-lg leading-none opacity-50 sm:block"
          >
            /
          </span>
          <Combobox
            mono
            showMetaInTrigger
            items={projectItems}
            value={project ?? ""}
            groupLabel={t("shell.projectsIn", { organization: currentOrganization.name })}
            searchPlaceholder={t("shell.searchProject")}
            emptyText={t("shell.noProject")}
            placeholder={t("shell.selectProject")}
            aria-label={t("shell.project")}
            onChange={(slug) =>
              navigate(switchProjectPath(pathname, currentOrganization.slug, slug))
            }
            actions={[
              {
                key: "all-projects",
                label: t("shell.allProjects"),
                icon: "Right",
                onSelect: () => navigate(organizationPath(currentOrganization.slug)),
              },
              {
                key: "new-project",
                label: t("shell.newProject"),
                icon: "PlusSmall",
                onSelect: () =>
                  navigate(organizationPath(currentOrganization.slug, "projects/new")),
              },
            ]}
          />
        </div>
      )}
      <div className="flex-1" />
      <button
        type="button"
        aria-label={t("shell.search")}
        onClick={onOpenPalette}
        className="flex h-8 cursor-pointer items-center gap-2 rounded bg-white/15 px-2.5 text-[13px] text-white hover:bg-white/25 sm:w-[260px] sm:pr-1.5"
      >
        <Icon name="Search" size={16} />
        <span className="hidden flex-1 whitespace-nowrap text-left opacity-90 sm:block">
          {t("shell.search")}
        </span>
        <span className="hidden gap-1 sm:inline-flex">
          <Kbd tone="inverse">{keys.join(keys[0] === "⌘" ? "" : " ")}</Kbd>
        </span>
      </button>
      <div className="hidden sm:block">
        <Tooltip title={`${t("shell.shortcuts")}  ?`} placement="bottom">
          <IconButton
            icon={<Icon name="Help" size={20} />}
            aria-label={t("shell.shortcuts")}
            onClick={onOpenShortcuts}
          />
        </Tooltip>
      </div>
      <div className="relative">
        <Tooltip title={`${t("shell.notifications")}  n`} placement="bottom">
          <IconButton
            icon={<Icon name="Mail" size={20} />}
            aria-label={
              unread > 0
                ? `${t("shell.notifications")}, ${t("shell.unread", { count: unread })}`
                : t("shell.notifications")
            }
            onClick={onOpenNotifications}
          />
        </Tooltip>
        {unread > 0 && (
          <span
            aria-hidden="true"
            className="pointer-events-none absolute right-0 top-0.5 min-w-4 rounded-full bg-[var(--danger-600)] px-1 text-center text-[11px] font-medium leading-4"
          >
            {unread}
          </span>
        )}
      </div>
    </header>
  );
}
