import { cn } from "@workspace/ui/lib/utils";
import { Alert, Button, Dialog, Input, message, Select, Toggle } from "@workspace/ui/taptap";
import { type ChangeEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { ApiFailure } from "~/api/client";
import {
  copyProject,
  createEmptyProject,
  importTree,
  type NewProjectInput,
} from "~/lib/project-create";
import { buildTree, type ImportTree } from "~/lib/sdd-tree";
import { isValidSlug, slugify } from "~/lib/slug";
import type { ProjectSummary } from "./project-card";

export type StartOption = "empty" | "import" | "copy";

export const START_OPTIONS: StartOption[] = ["empty", "import", "copy"];

export interface NewProjectDialogProps {
  organization: { slug: string; name: string };
  projects: ProjectSummary[];
  initialStart: StartOption;
  onClose: () => void;
}

interface Folder {
  name: string;
  tree: ImportTree;
}

const countDocuments = (tree: ImportTree) =>
  (tree.constitution ? 1 : 0) + tree.specs.reduce((sum, spec) => sum + spec.documents.length, 0);

export function NewProjectDialog({
  organization,
  projects,
  initialStart,
  onClose,
}: NewProjectDialogProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const ids = { name: useId(), slug: useId(), repository: useId() };
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [slugEdited, setSlugEdited] = useState(false);
  const [repository, setRepository] = useState("");
  const [start, setStart] = useState<StartOption>(initialStart);
  const [source, setSource] = useState("");
  const [folder, setFolder] = useState<Folder | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [problems, setProblems] = useState<string[]>([]);
  const [created, setCreated] = useState(false);

  const org = organization.slug;
  const address = `${org}/${slug}`;
  const sources = projects.filter((project) => Boolean(project.role));
  const slugInvalid = slug !== "" && !isValidSlug(slug);
  const folderProblems = folder?.tree.problems ?? [];
  const shownProblems = [...folderProblems, ...problems];
  const ready =
    name.trim() !== "" &&
    isValidSlug(slug) &&
    !busy &&
    (start === "copy" ? source !== "" : true) &&
    (start === "import" ? folder !== null && folderProblems.length === 0 : true);

  const changeName = (value: string) => {
    setName(value);
    if (!slugEdited) setSlug(slugify(value));
  };

  const changeSlug = (value: string) => {
    setSlug(value);
    setSlugEdited(value !== "");
  };

  const pickFolder = async (event: ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(event.target.files ?? []);
    event.target.value = "";
    if (files.length === 0) return;
    const tree = await buildTree(files);
    const root = files[0]?.webkitRelativePath.split("/")[0] ?? "";
    setFolder({ name: root, tree });
    setProblems([]);
  };

  const finish = (text: string) => {
    message.success(text);
    navigate(`/p/${address}`);
  };

  const submit = async () => {
    setBusy(true);
    setError(null);
    setProblems([]);
    const input: NewProjectInput = { slug, name: name.trim(), repository: repository.trim() };
    try {
      if (start === "copy") {
        await copyProject(org, source, input);
        finish(t("toast.projectCopied"));
        return;
      }
      if (!created) {
        await createEmptyProject(org, input);
        setCreated(true);
      }
      if (start === "empty" || !folder) {
        finish(t("toast.projectCreated"));
        return;
      }
      const refused = await importTree(org, slug, folder.tree, true);
      if (refused.length > 0) {
        setProblems(refused);
        return;
      }
      const failed = await importTree(org, slug, folder.tree, false);
      if (failed.length > 0) {
        setProblems(failed);
        return;
      }
      finish(t("toast.projectImported"));
    } catch (failure) {
      if (failure instanceof ApiFailure && failure.status === 401) {
        window.location.assign(
          `/auth/login?return_to=${encodeURIComponent(window.location.pathname)}`,
        );
        return;
      }
      setError(failure instanceof ApiFailure ? failure.message : t("errors.unexpected"));
    } finally {
      setBusy(false);
    }
  };

  const submitLabel = busy
    ? t("newProject.working")
    : created
      ? t("newProject.import")
      : start === "import"
        ? t("newProject.createAndImport")
        : start === "copy"
          ? t("newProject.copy")
          : t("newProject.create");

  return (
    <Dialog
      open
      title={t("newProject.title")}
      size={560}
      onCancel={onClose}
      style={{ maxHeight: "calc(100dvh - 48px)", overflowY: "auto" }}
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            {created ? t("newProject.close") : t("newProject.cancel")}
          </Button>
          <Button disabled={!ready} onClick={() => void submit()}>
            {submitLabel}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4 text-[var(--neutral-700)]">
        {created && problems.length > 0 && (
          <Alert status="warning" closable={false} showIcon>
            {t("newProject.createdWithProblems", { address })}
          </Alert>
        )}
        <div className="grid gap-3 [grid-template-columns:repeat(auto-fit,minmax(200px,1fr))]">
          <div className="flex flex-col gap-1.5">
            <label htmlFor={ids.name} className="text-sm">
              {t("newProject.name")}
            </label>
            <Input
              id={ids.name}
              value={name}
              disabled={created}
              placeholder={t("newProject.namePlaceholder")}
              onChange={changeName}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <label htmlFor={ids.slug} className="text-sm">
              {t("newProject.slug")}
            </label>
            <Input
              id={ids.slug}
              value={slug}
              disabled={created}
              error={slugInvalid}
              placeholder="payments-service"
              inputStyle={{ fontFamily: "var(--font-mono)" }}
              onChange={changeSlug}
            />
          </div>
        </div>
        <span className="-mt-2 text-xs leading-[18px] text-[var(--neutral-500)]">
          {t("newProject.slugPreview")}{" "}
          <span className="font-mono text-[var(--neutral-600)]">{address}</span>
        </span>
        {slugInvalid && (
          <span className="-mt-2 text-xs leading-[18px] text-[var(--danger-600)]">
            {t("newProject.slugInvalid")}
          </span>
        )}
        <div className="flex flex-col gap-1.5">
          <label htmlFor={ids.repository} className="text-sm">
            {t("newProject.repository")}{" "}
            <span className="text-[var(--neutral-500)]">{t("newProject.optional")}</span>
          </label>
          <Input
            id={ids.repository}
            value={repository}
            disabled={created}
            placeholder="https://github.com/acme/payments"
            onChange={setRepository}
          />
          <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
            {t("newProject.repositoryHint")}
          </span>
        </div>
        <div className="flex flex-col gap-2">
          <span id={`${ids.name}-start`} className="text-sm">
            {t("newProject.startFrom")}
          </span>
          <div
            role="radiogroup"
            aria-labelledby={`${ids.name}-start`}
            className="grid grid-cols-3 gap-2"
          >
            {START_OPTIONS.map((option) => (
              // biome-ignore lint/a11y/useSemanticElements: a card styled as a radio keeps the design
              <button
                key={option}
                type="button"
                role="radio"
                aria-checked={start === option}
                disabled={created}
                onClick={() => setStart(option)}
                className={cn(
                  "flex cursor-pointer flex-col gap-1 rounded border-0 p-3 text-left disabled:cursor-not-allowed disabled:opacity-60",
                  start === option
                    ? "bg-[var(--primary-100)] shadow-[inset_0_0_0_1px_var(--primary-600)]"
                    : "bg-[var(--neutral-50)] shadow-[inset_0_0_0_1px_var(--neutral-200)] hover:shadow-[inset_0_0_0_1px_var(--primary-500)]",
                )}
              >
                <span
                  className={cn(
                    "text-sm font-medium leading-[22px]",
                    start === option ? "text-[var(--primary-700)]" : "text-[var(--neutral-700)]",
                  )}
                >
                  {t(`newProject.start.${option}.label`)}
                </span>
                <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                  {t(`newProject.start.${option}.description`)}
                </span>
              </button>
            ))}
          </div>
          {start === "import" && (
            <div className="flex flex-col gap-2">
              <span className="text-xs leading-[18px] text-[var(--info-700)]">
                {t("newProject.dryRunHint")}
              </span>
              <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                {t("newProject.folderHelp")}
              </span>
              <div className="flex items-center gap-3">
                <Button
                  variant="outline"
                  size="small"
                  onClick={() => document.getElementById(`${ids.name}-folder`)?.click()}
                >
                  {folder ? t("newProject.changeFolder") : t("newProject.chooseFolder")}
                </Button>
                <input
                  id={`${ids.name}-folder`}
                  type="file"
                  multiple
                  className="hidden"
                  aria-label={folder ? t("newProject.changeFolder") : t("newProject.chooseFolder")}
                  onChange={(event) => void pickFolder(event)}
                  {...{ webkitdirectory: "" }}
                />
                {folder && (
                  <span className="text-xs text-[var(--neutral-600)]">
                    {t("newProject.folderRead", {
                      folder: folder.name,
                      specs: folder.tree.specs.length,
                      documents: countDocuments(folder.tree),
                    })}
                  </span>
                )}
              </div>
              {folder && folder.tree.skipped.length > 0 && (
                <span className="text-xs text-[var(--neutral-500)]">
                  {t("newProject.folderSkipped", { count: folder.tree.skipped.length })}
                </span>
              )}
            </div>
          )}
          {start === "copy" &&
            (sources.length > 0 ? (
              <div className="flex flex-col gap-1.5">
                <span className="text-sm">{t("newProject.copyFrom")}</span>
                <Select
                  aria-label={t("newProject.copyFrom")}
                  width="100%"
                  placeholder={t("newProject.copyPlaceholder")}
                  options={sources.map((project) => ({
                    label: project.name,
                    value: project.slug,
                  }))}
                  value={source || undefined}
                  onChange={(value) => setSource(String(value))}
                />
              </div>
            ) : (
              <span className="text-xs text-[var(--neutral-500)]">{t("newProject.copyNone")}</span>
            ))}
        </div>
        {shownProblems.length > 0 && (
          <div className="flex flex-col gap-1.5 rounded bg-[var(--danger-100)] p-3">
            <span className="text-sm font-medium text-[var(--danger-700)]">
              {t("newProject.problems")}
            </span>
            <ul
              aria-label={t("newProject.problems")}
              className="m-0 list-disc pl-5 font-mono text-xs leading-[18px] text-[var(--danger-700)]"
            >
              {shownProblems.map((problem) => (
                <li key={problem}>{problem}</li>
              ))}
            </ul>
          </div>
        )}
        {error && (
          <p role="alert" className="m-0 text-sm text-[var(--danger-600)]">
            {error}
          </p>
        )}
        <div className="flex items-center gap-4 rounded bg-[var(--neutral-50)] px-3.5 py-3">
          <div className="flex flex-1 flex-col">
            <span className="text-sm leading-[22px]">
              {t("newProject.inherit", { organization: organization.name })}
            </span>
            <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
              {t("newProject.inheritNote")}
            </span>
          </div>
          <Toggle
            disabled
            checked={false}
            aria-label={t("newProject.inherit", { organization: organization.name })}
          />
        </div>
        <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
          {t("newProject.owner")}
        </span>
      </div>
    </Dialog>
  );
}
