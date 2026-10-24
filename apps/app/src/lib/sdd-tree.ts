import { SPEC_KINDS, type SpecKind } from "./format";
import { isValidSlug } from "./slug";

export interface PickedFile {
  webkitRelativePath: string;
  size: number;
  text: () => Promise<string>;
}

export interface TreeFile {
  relative_path: string;
  body: string;
}

export interface TreeSpec {
  folder: string;
  number: number;
  slug: string;
  documents: [SpecKind, TreeFile][];
}

export interface ImportTree {
  constitution: TreeFile | null;
  specs: TreeSpec[];
  skipped: string[];
  problems: string[];
}

export const CONSTITUTION_PATH = "docs/constitution.md";
export const MAX_BODY_BYTES = 1_048_576;

const BRANCH_PATTERN = /^(\d+)-(.+)$/;

const compare = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

export function parseSpecFolder(name: string): { number: number; slug: string } | null {
  const match = BRANCH_PATTERN.exec(name);
  if (!match) return null;
  const number = Number(match[1]);
  const slug = match[2] ?? "";
  if (!Number.isSafeInteger(number) || number < 1 || !isValidSlug(slug)) return null;
  return `${String(number).padStart(3, "0")}-${slug}` === name ? { number, slug } : null;
}

const kindOf = (fileName: string): SpecKind | undefined =>
  SPEC_KINDS.find((kind) => `${kind}.md` === fileName);

interface Entry {
  path: string;
  file: PickedFile;
}

async function readDocument(entry: Entry, problems: string[]): Promise<TreeFile | null> {
  if (entry.file.size > MAX_BODY_BYTES) {
    problems.push(
      `${entry.path}: ${entry.file.size} bytes exceeds the limit of ${MAX_BODY_BYTES} bytes`,
    );
    return null;
  }
  return { relative_path: entry.path, body: await entry.file.text() };
}

export async function buildTree(files: ArrayLike<PickedFile>): Promise<ImportTree> {
  const tree: ImportTree = { constitution: null, specs: [], skipped: [], problems: [] };
  const entries = Array.from(files, (file): Entry & { root: string } => {
    const [root = "", ...rest] = file.webkitRelativePath.split("/");
    return { root, path: rest.join("/"), file };
  });
  const root = entries[0]?.root ?? "";

  const constitution = entries.find((entry) => entry.path === CONSTITUTION_PATH);
  if (constitution) {
    tree.constitution = await readDocument(constitution, tree.problems);
  }

  const folders = new Map<string, Entry[]>();
  for (const entry of entries) {
    const [top, folder, ...rest] = entry.path.split("/");
    if (top !== "specs" || folder === undefined) continue;
    if (rest.length === 0) {
      tree.skipped.push(entry.path);
      continue;
    }
    folders.set(folder, [...(folders.get(folder) ?? []), entry]);
  }

  for (const folder of [...folders.keys()].sort(compare)) {
    const parsed = parseSpecFolder(folder);
    if (!parsed) {
      tree.problems.push(`specs/${folder}: folder name is not a spec branch name such as 001-name`);
      continue;
    }
    const sameNumber = tree.specs.find((spec) => spec.number === parsed.number);
    if (sameNumber) {
      tree.problems.push(
        `specs/${folder}: number ${String(parsed.number).padStart(3, "0")} is also used by specs/${sameNumber.folder}`,
      );
      continue;
    }
    const sameSlug = tree.specs.find((spec) => spec.slug === parsed.slug);
    if (sameSlug) {
      tree.problems.push(
        `specs/${folder}: slug "${parsed.slug}" is also used by specs/${sameSlug.folder}`,
      );
      continue;
    }
    const documents: [SpecKind, TreeFile][] = [];
    const directories = new Set<string>();
    for (const entry of folders.get(folder) ?? []) {
      const [, , name = "", ...deeper] = entry.path.split("/");
      if (deeper.length > 0) {
        directories.add(`specs/${folder}/${name}/`);
        continue;
      }
      const kind = kindOf(name);
      if (!kind) {
        tree.skipped.push(entry.path);
        continue;
      }
      const document = await readDocument(entry, tree.problems);
      if (document) documents.push([kind, document]);
    }
    tree.skipped.push(...directories);
    documents.sort(([a], [b]) => SPEC_KINDS.indexOf(a) - SPEC_KINDS.indexOf(b));
    tree.specs.push({ folder, number: parsed.number, slug: parsed.slug, documents });
  }

  if (folders.size === 0 && !constitution) {
    tree.problems.push(`${root}: no ${CONSTITUTION_PATH} and no spec folder in specs/`);
  }
  tree.skipped.sort(compare);
  return tree;
}
