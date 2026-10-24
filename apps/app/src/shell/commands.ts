import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  ACCOUNT_ENTRIES,
  type NavIcon,
  ORGANIZATION_ACCOUNT_ENTRIES,
  organizationPath,
  PROJECT_MAIN,
  PROJECT_SDD,
  PROJECT_SECTION,
  projectPath,
} from "./navigation";
import { specLabel } from "./platform";
import type { ShellActiveSpec, ShellSpec } from "./types";

export interface Command {
  id: string;
  label: string;
  sub?: string;
  icon: NavIcon;
  mono?: boolean;
  keys?: string[];
  run: () => void;
}

export interface CommandGroup {
  id: "suggested" | "specs" | "navigation";
  commands: Command[];
}

export interface CommandContext {
  org?: string;
  project?: string;
  specs: ShellSpec[];
  active?: ShellActiveSpec;
  onOpenNotifications: () => void;
  onOpenShortcuts: () => void;
}

export const SPECS_WITHOUT_QUERY = 5;

export function useCommands(context: CommandContext, close: () => void): CommandGroup[] {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { org, project, specs, active } = context;
  const open = (path: string) => () => {
    close();
    navigate(path);
  };
  const act = (action: () => void) => () => {
    close();
    action();
  };

  const suggested: Command[] = [];
  if (org && project && active?.nextTask) {
    suggested.push({
      id: "next-task",
      label: active.nextTask.id,
      sub: t("palette.nextTask", { description: active.nextTask.description }),
      icon: "Check",
      mono: true,
      run: open(projectPath(org, project, `specs/${active.number}/tasks`)),
    });
  }
  if (org && project && active) {
    suggested.push({
      id: "active-spec",
      label: specLabel(active.number),
      sub: `${t("palette.activeSpec")} · ${active.slug}`,
      icon: "Focus",
      mono: true,
      run: open(projectPath(org, project, `specs/${active.number}`)),
    });
  }
  if (org) {
    suggested.push({
      id: "new-project",
      label: t("shell.newProject"),
      icon: "PlusSmall",
      run: open(organizationPath(org, "projects/new")),
    });
  }
  suggested.push(
    {
      id: "notifications",
      label: t("palette.openNotifications"),
      icon: "Mail",
      keys: ["n"],
      run: act(context.onOpenNotifications),
    },
    {
      id: "shortcuts",
      label: t("shell.shortcuts"),
      icon: "Help",
      keys: ["?"],
      run: act(context.onOpenShortcuts),
    },
  );

  const specCommands: Command[] =
    org && project
      ? [...specs]
          .sort((a, b) => b.number - a.number)
          .map((spec) => ({
            id: `spec-${spec.number}`,
            label: specLabel(spec.number),
            sub: `${spec.slug} · ${t(`spec.statuses.${spec.status as "draft"}`)}`,
            icon: "File",
            mono: true,
            run: open(projectPath(org, project, `specs/${spec.number}`)),
          }))
      : [];

  const navigation: Command[] = [];
  if (org && project) {
    for (const entry of [...PROJECT_MAIN, ...PROJECT_SDD, ...PROJECT_SECTION]) {
      navigation.push({
        id: `project-${entry.id}`,
        label: t(entry.label),
        sub: project,
        icon: entry.icon,
        keys: entry.chord ? ["g", entry.chord] : undefined,
        run: open(projectPath(org, project, entry.section)),
      });
    }
  }
  for (const entry of ACCOUNT_ENTRIES) {
    navigation.push({
      id: `account-${entry.id}`,
      label: t(entry.label),
      icon: entry.icon,
      keys: entry.chord ? ["g", entry.chord] : undefined,
      run: open(entry.path),
    });
  }
  if (org) {
    navigation.push(
      {
        id: "org-projects",
        label: t("palette.organizationProjects"),
        sub: org,
        icon: "BarChart",
        run: open(organizationPath(org)),
      },
      {
        id: "org-members",
        label: t("palette.organizationMembers"),
        sub: org,
        icon: "User",
        run: open(organizationPath(org, "members")),
      },
    );
    for (const entry of ORGANIZATION_ACCOUNT_ENTRIES) {
      navigation.push({
        id: `org-${entry.id}`,
        label: t(entry.label),
        sub: org,
        icon: entry.icon,
        run: open(organizationPath(org, entry.section)),
      });
    }
  }

  return [
    { id: "suggested", commands: suggested },
    { id: "specs", commands: specCommands },
    { id: "navigation", commands: navigation },
  ];
}

export const filterGroups = (groups: CommandGroup[], query: string): CommandGroup[] => {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return groups
      .map((group) =>
        group.id === "specs"
          ? { ...group, commands: group.commands.slice(0, SPECS_WITHOUT_QUERY) }
          : group,
      )
      .filter((group) => group.commands.length > 0);
  }
  return groups
    .filter((group) => group.id !== "suggested")
    .map((group) => ({
      ...group,
      commands: group.commands.filter((command) =>
        `${command.label} ${command.sub ?? ""}`.toLowerCase().includes(needle),
      ),
    }))
    .filter((group) => group.commands.length > 0);
};
