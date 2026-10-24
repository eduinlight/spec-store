import { type ActivityRow, filterOf, mergeRows, oldestId, sentenceOf } from "./activity";

const row = (overrides: Partial<ActivityRow>): ActivityRow => ({
  audit_id: 1,
  actor: "dana",
  client: "Cursor",
  action: "document.put",
  target: "004/spec@14",
  result: "ok",
  detail: null,
  at: "2026-10-24T10:00:00.000Z",
  summary_key: "document.put",
  resolved_by_audit_id: null,
  ...overrides,
});

test("filters other than the known four fall back to all", () => {
  expect(filterOf("tasks")).toBe("tasks");
  expect(filterOf("security")).toBe("security");
  expect(filterOf("nope")).toBe("all");
  expect(filterOf(null)).toBe("all");
});

test("a finished task names the task and its commit", () => {
  expect(
    sentenceOf(
      row({ action: "task.done", summary_key: "task.done", target: "004/T07@a1b2c3d" }),
      [],
    ),
  ).toEqual({
    key: "summary.taskDone",
    values: { task: "T07", commit: "a1b2c3d", spec: "004" },
  });
});

test("a document write names the document and revision", () => {
  expect(sentenceOf(row({}), [])).toEqual({
    key: "summary.documentPut",
    values: { document: "004/spec", revision: "r14" },
  });
});

test("status, member and repository changes are read from their targets", () => {
  expect(
    sentenceOf(
      row({ action: "spec.status", summary_key: "spec.status", target: "004:draft→clarified" }),
      [],
    ),
  ).toEqual({
    key: "summary.specStatus",
    values: { number: "004", from: "draft", to: "clarified" },
  });
  expect(
    sentenceOf(row({ action: "member.set", summary_key: "member.set", target: "rita:editor" }), []),
  ).toEqual({ key: "summary.memberSet", values: { handle: "rita", role: "editor" } });
  expect(
    sentenceOf(
      row({ action: "project.repo", summary_key: "project.repo", target: "demo cleared" }),
      [],
    ).key,
  ).toBe("summary.projectRepoCleared");
});

test("an unknown action still reads as a sentence", () => {
  expect(sentenceOf(row({ action: "x.y", summary_key: "x.y", target: "t" }), [])).toEqual({
    key: "summary.fallback",
    values: { action: "x.y", target: "t" },
  });
});

test("a denied write says what was tried", () => {
  expect(
    sentenceOf(row({ result: "denied", summary_key: "denied", target: "004/spec" }), []).key,
  ).toBe("denied");
  expect(
    sentenceOf(row({ action: "x.y", result: "denied", summary_key: "denied", target: "t" }), [])
      .key,
  ).toBe("deniedFallback");
});

test("a conflict shows the revision it was based on and the current one", () => {
  const conflict = row({
    result: "conflict",
    summary_key: "conflict",
    target: "004/spec",
    detail: "expected r12, current r13",
  });
  expect(sentenceOf(conflict, [])).toEqual({
    key: "conflict",
    values: { document: "004/spec", expected: "r12", current: "r13", saved: "" },
  });
  expect(sentenceOf({ ...conflict, detail: "expected none, current r13" }, []).key).toBe(
    "conflictNoBase",
  );
});

test("a resolved conflict points to the revision that was finally saved when it is loaded", () => {
  const saved = row({ audit_id: 5, target: "004/spec@15" });
  const conflict = row({
    audit_id: 4,
    result: "conflict",
    summary_key: "conflict_resolved",
    target: "004/spec",
    detail: "expected r12, current r13",
    resolved_by_audit_id: 5,
  });
  expect(sentenceOf(conflict, [saved, conflict])).toEqual({
    key: "conflictResolved",
    values: { document: "004/spec", expected: "r12", current: "r13", saved: " (r15)" },
  });
  expect(sentenceOf(conflict, [conflict]).values.saved).toBe("");
});

test("pages merge newest first without duplicates", () => {
  const merged = mergeRows(
    [row({ audit_id: 5 }), row({ audit_id: 4 })],
    [row({ audit_id: 4 }), row({ audit_id: 3 })],
  );
  expect(merged.map((item) => item.audit_id)).toEqual([5, 4, 3]);
  expect(oldestId(merged)).toBe(3);
  expect(oldestId([])).toBeUndefined();
});
