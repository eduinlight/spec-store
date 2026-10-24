import { buildTree, type PickedFile } from "./sdd-tree";

const pick = (path: string, body = "body", size = body.length): PickedFile => ({
  webkitRelativePath: `repo/${path}`,
  size,
  text: async () => body,
});

test("reads docs/constitution.md and the documents of each spec folder", async () => {
  const tree = await buildTree([
    pick("docs/constitution.md", "Version 1.0"),
    pick("specs/001-first/tasks.md", "tasks"),
    pick("specs/001-first/spec.md", "spec"),
    pick("specs/002-second/plan.md", "plan"),
  ]);
  expect(tree).toEqual({
    constitution: { relative_path: "docs/constitution.md", body: "Version 1.0" },
    specs: [
      {
        folder: "001-first",
        number: 1,
        slug: "first",
        documents: [
          ["spec", { relative_path: "specs/001-first/spec.md", body: "spec" }],
          ["tasks", { relative_path: "specs/001-first/tasks.md", body: "tasks" }],
        ],
      },
      {
        folder: "002-second",
        number: 2,
        slug: "second",
        documents: [["plan", { relative_path: "specs/002-second/plan.md", body: "plan" }]],
      },
    ],
    skipped: [],
    problems: [],
  });
});

test("ignores everything outside docs/constitution.md and specs without reading it", async () => {
  const read = vi.fn(async () => "x");
  const tree = await buildTree([
    pick("docs/constitution.md", "Version 1.0"),
    pick("node_modules/pkg/index.js"),
    pick("README.md"),
    pick("docs/other.md"),
    { webkitRelativePath: "repo/src/main.rs", size: 1, text: read },
  ]);
  expect(tree.specs).toEqual([]);
  expect(tree.skipped).toEqual([]);
  expect(tree.problems).toEqual([]);
  expect(read).not.toHaveBeenCalled();
});

test("skips files that are not spec documents and reports them", async () => {
  const tree = await buildTree([
    pick("specs/001-first/spec.md"),
    pick("specs/001-first/notes.md"),
    pick("specs/001-first/assets/diagram.png"),
    pick("specs/README.md"),
  ]);
  expect(tree.skipped).toEqual([
    "specs/001-first/assets/",
    "specs/001-first/notes.md",
    "specs/README.md",
  ]);
  expect(tree.specs[0]?.documents).toHaveLength(1);
});

test("reports spec folders whose name is not a spec branch name", async () => {
  const tree = await buildTree([
    pick("specs/first/spec.md"),
    pick("specs/1-first/spec.md"),
    pick("specs/001-First/spec.md"),
    pick("specs/000-zero/spec.md"),
  ]);
  expect(tree.specs).toEqual([]);
  expect(tree.problems).toEqual([
    "specs/000-zero: folder name is not a spec branch name such as 001-name",
    "specs/001-First: folder name is not a spec branch name such as 001-name",
    "specs/1-first: folder name is not a spec branch name such as 001-name",
    "specs/first: folder name is not a spec branch name such as 001-name",
  ]);
});

test("reports repeated numbers and slugs", async () => {
  const tree = await buildTree([
    pick("specs/001-same/spec.md"),
    pick("specs/001-twin/spec.md"),
    pick("specs/002-same/spec.md"),
  ]);
  expect(tree.specs.map((spec) => spec.folder)).toEqual(["001-same"]);
  expect(tree.problems).toEqual([
    "specs/001-twin: number 001 is also used by specs/001-same",
    'specs/002-same: slug "same" is also used by specs/001-same',
  ]);
});

test("reports an empty folder", async () => {
  const tree = await buildTree([pick("README.md")]);
  expect(tree.problems).toEqual(["repo: no docs/constitution.md and no spec folder in specs/"]);
});

test("reports documents over the size limit and does not read them", async () => {
  const read = vi.fn(async () => "x");
  const tree = await buildTree([
    pick("docs/constitution.md", "Version 1.0"),
    { webkitRelativePath: "repo/specs/001-big/spec.md", size: 1_048_577, text: read },
  ]);
  expect(tree.problems).toEqual([
    "specs/001-big/spec.md: 1048577 bytes exceeds the limit of 1048576 bytes",
  ]);
  expect(read).not.toHaveBeenCalled();
  expect(tree.specs[0]?.documents).toEqual([]);
});

test("treats the first path segment as the chosen folder", async () => {
  const tree = await buildTree([
    { webkitRelativePath: "docs/constitution.md", size: 3, text: async () => "abc" },
  ]);
  expect(tree.constitution).toEqual(null);
  expect(tree.problems).toEqual(["docs: no docs/constitution.md and no spec folder in specs/"]);
});
