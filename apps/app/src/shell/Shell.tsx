import { Drawer, Icon } from "@workspace/ui/taptap";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation } from "react-router";
import type { Language } from "~/lib/language";
import type { ShellData } from "./data";
import { Header } from "./Header";
import { accountMode, parsePlace } from "./navigation";
import { specLabel } from "./platform";
import { Sidebar } from "./Sidebar";
import { changeLanguage, signOut } from "./session-actions";
import type { ShellOrganization, ShellProject } from "./types";
import { UserMenu } from "./UserMenu";

interface Memory {
  org?: string;
  project?: ShellProject;
  backPath?: string;
}

export interface ShellProps {
  data: ShellData;
  children: ReactNode;
}

const UNREAD_NOTIFICATIONS = 0;

export function Shell({ data, children }: ShellProps) {
  const { t, i18n } = useTranslation();
  const { pathname, search } = useLocation();
  const place = parsePlace(pathname);
  const inAccount = accountMode(place);
  const [memory, setMemory] = useState<Memory>({});
  const [notificationsOpen, setNotificationsOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const placeProject = data.projects.find((item) => item.slug === place.project);
  const placeProjectName = placeProject?.name;
  const placeProjectRole = placeProject?.role;

  useEffect(() => {
    if (inAccount || !place.org || (place.area !== "project" && place.area !== "organization")) {
      return;
    }
    const org = place.org;
    const project =
      place.area === "project" && place.project
        ? { slug: place.project, name: placeProjectName ?? place.project, role: placeProjectRole }
        : undefined;
    const backPath = pathname + search;
    setMemory((previous) =>
      previous.org === org &&
      previous.backPath === backPath &&
      previous.project?.slug === project?.slug &&
      previous.project?.name === project?.name
        ? previous
        : { org, project, backPath },
    );
  }, [
    inAccount,
    place.area,
    place.org,
    place.project,
    pathname,
    search,
    placeProjectName,
    placeProjectRole,
  ]);

  const orgSlug = place.org ?? memory.org ?? data.organizations[0]?.slug;
  const knownOrganization = data.organizations.find((item) => item.slug === orgSlug);
  const organization: ShellOrganization | undefined =
    knownOrganization ?? (orgSlug ? { slug: orgSlug, name: orgSlug, role: null } : undefined);
  const organizations =
    organization && !knownOrganization ? [organization, ...data.organizations] : data.organizations;
  const projectSlug = place.project ?? (inAccount ? memory.project?.slug : undefined);
  const projects =
    place.org && data.projects.length > 0
      ? data.projects
      : memory.project && memory.org === orgSlug
        ? [memory.project]
        : [];
  const activeSpec =
    place.area === "project" && data.active
      ? `${specLabel(data.active.number)} ${data.active.slug}`
      : undefined;
  const back = memory.backPath
    ? {
        path: memory.backPath,
        label: memory.backPath.startsWith("/p/")
          ? (memory.project?.slug ?? "")
          : (data.organizations.find((item) => item.slug === memory.org)?.name ?? memory.org ?? ""),
      }
    : { path: "/", label: t("app.name") };

  const sidebar = (onNavigate?: () => void) => (
    <>
      <Sidebar place={place} organization={organization} back={back} onNavigate={onNavigate} />
      <UserMenu
        person={data.person}
        organization={organization}
        language={i18n.language}
        onLanguage={(language: Language) => void changeLanguage(i18n, language)}
        onSignOut={() => void signOut()}
        onNavigate={onNavigate}
      />
    </>
  );

  return (
    <div className="flex h-dvh flex-col bg-[var(--neutral-100)] text-[var(--neutral-700)]">
      <Header
        organizations={organizations}
        projects={projects}
        org={orgSlug}
        project={projectSlug}
        activeSpec={activeSpec}
        unread={UNREAD_NOTIFICATIONS}
        onOpenPalette={() => undefined}
        onOpenShortcuts={() => undefined}
        onOpenNotifications={() => setNotificationsOpen(true)}
        onOpenMenu={() => setMenuOpen(true)}
        onOrganizationChange={(slug) => setMemory({ org: slug })}
      />
      <div className="flex min-h-0 flex-1">
        <aside className="hidden w-[220px] flex-none flex-col border-r border-[var(--neutral-200)] bg-white min-[880px]:flex">
          {sidebar()}
        </aside>
        <main className="min-w-0 flex-1 overflow-y-auto px-4 pb-12 pt-4 sm:px-6">
          <div className="mx-auto max-w-[1280px]">{children}</div>
        </main>
      </div>
      <Drawer
        open={notificationsOpen}
        title={t("shell.notifications")}
        footer={null}
        onClose={() => setNotificationsOpen(false)}
      >
        <div className="flex flex-col items-center gap-3 py-16 text-center text-[var(--neutral-500)]">
          <Icon name="Mail" size={32} />
          <p>{t("shell.notificationsEmpty")}</p>
        </div>
      </Drawer>
      <Drawer
        open={menuOpen}
        placement="left"
        width={280}
        title={t("app.name")}
        footer={null}
        onClose={() => setMenuOpen(false)}
      >
        <div className="-m-6 flex h-[calc(100%+3rem)] min-h-0 flex-col">
          {sidebar(() => setMenuOpen(false))}
        </div>
      </Drawer>
    </div>
  );
}
