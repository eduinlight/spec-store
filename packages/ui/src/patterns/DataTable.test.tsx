import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type ComponentProps, useState } from "react";
import { DataTable, type DataTableColumn } from "./DataTable";

interface Entry {
  id: string;
  time: number;
  actor: string;
  action: string;
}

const entries: Entry[] = [
  { id: "e1", time: 3, actor: "Ada", action: "spec.create" },
  { id: "e2", time: 1, actor: "Grace", action: "spec.update" },
  { id: "e3", time: 2, actor: "Linus", action: "task.approve" },
  { id: "e4", time: 5, actor: "Alan", action: "spec.delete" },
  { id: "e5", time: 4, actor: "Edsger", action: "task.claim" },
];

const columns: DataTableColumn<Entry>[] = [
  { key: "time", title: "Time", sortable: true },
  { key: "actor", title: "Actor", sortable: true, accessor: (row) => row.actor },
  {
    key: "action",
    title: "Action",
    hideable: true,
    render: (row) => <code>{row.action}</code>,
  },
  { key: "id", title: "Id", hideable: true },
];

const many: Entry[] = Array.from({ length: 25 }, (_, index) => ({
  id: `m${index + 1}`,
  time: index + 1,
  actor: `Actor ${index + 1}`,
  action: "spec.update",
}));

const renderTable = (props: Partial<ComponentProps<typeof DataTable<Entry>>> = {}) =>
  render(<DataTable columns={columns} data={entries} rowKey={(row) => row.id} {...props} />);

const bodyRows = () =>
  within(screen.getAllByRole("rowgroup")[1] as HTMLElement).getAllByRole("row");
const firstColumn = () => bodyRows().map((row) => within(row).getAllByRole("cell")[1]?.textContent);

function Harness() {
  const [query, setQuery] = useState("");
  return (
    <>
      <input aria-label="filter" value={query} onChange={(event) => setQuery(event.target.value)} />
      <DataTable
        columns={columns}
        data={many.filter((row) => row.actor.includes(query))}
        rowKey={(row) => row.id}
        defaultPageSize={10}
      />
    </>
  );
}

describe("DataTable", () => {
  it("renders a real table with headers, rows and custom cells", () => {
    renderTable({ label: "Audit log" });
    expect(screen.getByRole("table", { name: "Audit log" })).toBeInTheDocument();
    expect(screen.getAllByRole("columnheader")).toHaveLength(5);
    expect(bodyRows()).toHaveLength(5);
    expect(screen.getByText("spec.create").tagName).toBe("CODE");
    expect(screen.getByRole("columnheader", { name: "Time" }).style.background).toBe(
      "var(--neutral-50)",
    );
  });

  it("shows the empty text without rows", () => {
    renderTable({ data: [] });
    expect(screen.getByText("No results.")).toBeInTheDocument();
    expect(screen.getByText("0 of 0 row(s) selected.")).toBeInTheDocument();
    expect(screen.getByText("Page 1 of 1")).toBeInTheDocument();
  });

  it("uses a custom empty text", () => {
    renderTable({ data: [], emptyText: "Nothing yet" });
    expect(screen.getByText("Nothing yet")).toBeInTheDocument();
  });

  it("selects rows, reports the keys and counts them", async () => {
    const onSelectionChange = vi.fn();
    renderTable({ onSelectionChange });
    const boxes = screen.getAllByRole("checkbox", { name: "Select row" });
    await userEvent.click(boxes[0] as HTMLElement);
    await userEvent.click(boxes[2] as HTMLElement);
    expect(onSelectionChange).toHaveBeenLastCalledWith(["e1", "e3"]);
    expect(screen.getByText("2 of 5 row(s) selected.")).toBeInTheDocument();
    await userEvent.click(boxes[0] as HTMLElement);
    expect(onSelectionChange).toHaveBeenLastCalledWith(["e3"]);
    expect(screen.getByText("1 of 5 row(s) selected.")).toBeInTheDocument();
  });

  it("makes the select-all checkbox indeterminate for some rows and checked for all", async () => {
    renderTable();
    const all = screen.getByRole("checkbox", { name: "Select all rows on this page" });
    expect(all).not.toBeChecked();
    expect((all as HTMLInputElement).indeterminate).toBe(false);
    await userEvent.click(
      screen.getAllByRole("checkbox", { name: "Select row" })[0] as HTMLElement,
    );
    expect((all as HTMLInputElement).indeterminate).toBe(true);
    await userEvent.click(all);
    expect(all).toBeChecked();
    expect((all as HTMLInputElement).indeterminate).toBe(false);
    expect(screen.getByText("5 of 5 row(s) selected.")).toBeInTheDocument();
    await userEvent.click(all);
    expect(screen.getByText("0 of 5 row(s) selected.")).toBeInTheDocument();
  });

  it("selects only the rows of the current page", async () => {
    const onSelectionChange = vi.fn();
    renderTable({ data: many, defaultPageSize: 5, onSelectionChange });
    await userEvent.click(screen.getByRole("checkbox", { name: "Select all rows on this page" }));
    expect(onSelectionChange).toHaveBeenLastCalledWith(["m1", "m2", "m3", "m4", "m5"]);
    expect(screen.getByText("5 of 25 row(s) selected.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Go to next page" }));
    expect(
      screen.getByRole("checkbox", { name: "Select all rows on this page" }),
    ).not.toBeChecked();
    await userEvent.click(screen.getByRole("checkbox", { name: "Select all rows on this page" }));
    expect(screen.getByText("10 of 25 row(s) selected.")).toBeInTheDocument();
  });

  it("supports a controlled selection", async () => {
    const onSelectionChange = vi.fn();
    renderTable({ selectedKeys: ["e2"], onSelectionChange });
    expect(screen.getByText("1 of 5 row(s) selected.")).toBeInTheDocument();
    await userEvent.click(
      screen.getAllByRole("checkbox", { name: "Select row" })[0] as HTMLElement,
    );
    expect(onSelectionChange).toHaveBeenCalledWith(["e2", "e1"]);
    expect(screen.getByText("1 of 5 row(s) selected.")).toBeInTheDocument();
  });

  it("can hide the selection column", () => {
    renderTable({ selectable: false });
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    expect(screen.queryByText(/row\(s\) selected/)).not.toBeInTheDocument();
  });

  it("sorts ascending, then descending, with aria-sort and an indicator", async () => {
    renderTable();
    const header = screen.getByRole("columnheader", { name: "Time" });
    expect(header).toHaveAttribute("aria-sort", "none");
    await userEvent.click(within(header).getByRole("button"));
    expect(header).toHaveAttribute("aria-sort", "ascending");
    expect(firstColumn()).toEqual(["1", "2", "3", "4", "5"]);
    await userEvent.click(within(header).getByRole("button"));
    expect(header).toHaveAttribute("aria-sort", "descending");
    expect(firstColumn()).toEqual(["5", "4", "3", "2", "1"]);
    await userEvent.click(within(header).getByRole("button"));
    expect(header).toHaveAttribute("aria-sort", "ascending");
  });

  it("sorts with Enter and Space and resets the previous column", async () => {
    renderTable();
    const time = screen.getByRole("columnheader", { name: "Time" });
    const actor = screen.getByRole("columnheader", { name: "Actor" });
    within(time).getByRole("button").focus();
    await userEvent.keyboard("{Enter}");
    expect(time).toHaveAttribute("aria-sort", "ascending");
    within(actor).getByRole("button").focus();
    await userEvent.keyboard(" ");
    expect(actor).toHaveAttribute("aria-sort", "ascending");
    expect(time).toHaveAttribute("aria-sort", "none");
    expect(bodyRows().map((row) => within(row).getAllByRole("cell")[2]?.textContent)).toEqual([
      "Ada",
      "Alan",
      "Edsger",
      "Grace",
      "Linus",
    ]);
  });

  it("does not mark unsortable headers", () => {
    renderTable();
    expect(screen.getByRole("columnheader", { name: "Id" })).not.toHaveAttribute("aria-sort");
  });

  it("sorts with a compare function", async () => {
    const byLength = (a: Entry, b: Entry) => a.action.length - b.action.length;
    renderTable({
      columns: [{ key: "action", title: "Action", sortable: true, compare: byLength }],
      selectable: false,
    });
    await userEvent.click(screen.getByRole("button", { name: /Action/ }));
    expect(bodyRows().map((row) => row.textContent)).toEqual([
      "task.claim",
      "spec.create",
      "spec.update",
      "spec.delete",
      "task.approve",
    ]);
    await userEvent.click(screen.getByRole("button", { name: /Action/ }));
    expect(bodyRows()[0]).toHaveTextContent("task.approve");
  });

  it("supports a controlled sort", async () => {
    const onSortChange = vi.fn();
    renderTable({ sort: { key: "time", direction: "desc" }, onSortChange });
    expect(firstColumn()).toEqual(["5", "4", "3", "2", "1"]);
    await userEvent.click(
      within(screen.getByRole("columnheader", { name: "Time" })).getByRole("button"),
    );
    expect(onSortChange).toHaveBeenCalledWith({ key: "time", direction: "asc" });
    expect(firstColumn()).toEqual(["5", "4", "3", "2", "1"]);
  });

  it("starts from a default sort", () => {
    renderTable({ defaultSort: { key: "time", direction: "asc" } });
    expect(firstColumn()).toEqual(["1", "2", "3", "4", "5"]);
  });

  it("paginates and shows the page text", async () => {
    renderTable({ data: many, defaultPageSize: 10 });
    expect(bodyRows()).toHaveLength(10);
    expect(screen.getByText("Page 1 of 3")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Go to first page" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Go to previous page" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Go to next page" }));
    expect(screen.getByText("Page 2 of 3")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Go to last page" }));
    expect(screen.getByText("Page 3 of 3")).toBeInTheDocument();
    expect(bodyRows()).toHaveLength(5);
    expect(screen.getByRole("button", { name: "Go to next page" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Go to last page" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Go to previous page" }));
    expect(screen.getByText("Page 2 of 3")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Go to first page" }));
    expect(screen.getByText("Page 1 of 3")).toBeInTheDocument();
  });

  it("changes the page size and returns to the first page", async () => {
    const onPageSizeChange = vi.fn();
    renderTable({ data: many, defaultPageSize: 10, onPageSizeChange });
    await userEvent.click(screen.getByRole("button", { name: "Go to last page" }));
    await userEvent.click(screen.getByRole("combobox", { name: "Rows per page" }));
    await userEvent.click(screen.getByRole("option", { name: "20" }));
    expect(onPageSizeChange).toHaveBeenCalledWith(20);
    expect(bodyRows()).toHaveLength(20);
    expect(screen.getByText("Page 1 of 2")).toBeInTheDocument();
  });

  it("returns to the last page when the data shrinks below the current page", async () => {
    const onPageChange = vi.fn();
    const { rerender } = render(
      <DataTable
        columns={columns}
        data={many}
        rowKey={(row) => row.id}
        defaultPageSize={10}
        onPageChange={onPageChange}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "Go to last page" }));
    expect(screen.getByText("Page 3 of 3")).toBeInTheDocument();
    rerender(
      <DataTable
        columns={columns}
        data={many.slice(0, 12)}
        rowKey={(row) => row.id}
        defaultPageSize={10}
        onPageChange={onPageChange}
      />,
    );
    expect(screen.getByText("Page 2 of 2")).toBeInTheDocument();
    expect(bodyRows()).toHaveLength(2);
    expect(onPageChange).toHaveBeenLastCalledWith(2);
    rerender(
      <DataTable
        columns={columns}
        data={[]}
        rowKey={(row) => row.id}
        defaultPageSize={10}
        onPageChange={onPageChange}
      />,
    );
    expect(screen.getByText("Page 1 of 1")).toBeInTheDocument();
  });

  it("corrects a controlled page that is beyond the last one", () => {
    const onPageChange = vi.fn();
    renderTable({ data: many.slice(0, 12), page: 9, defaultPageSize: 10, onPageChange });
    expect(screen.getByText("Page 2 of 2")).toBeInTheDocument();
    expect(onPageChange).toHaveBeenCalledWith(2);
  });

  it("keeps a selection that is outside the filtered data out of the count", () => {
    renderTable({ selectedKeys: ["e1", "gone"] });
    expect(screen.getByText("1 of 5 row(s) selected.")).toBeInTheDocument();
  });

  it("hides and shows columns from the View menu", async () => {
    const onHiddenKeysChange = vi.fn();
    renderTable({ onHiddenKeysChange });
    const view = screen.getByRole("button", { name: "View" });
    expect(view).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(view);
    expect(view).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Toggle columns")).toBeInTheDocument();
    expect(screen.getAllByRole("checkbox", { name: /Action|Id/ })).toHaveLength(2);
    expect(screen.queryByRole("checkbox", { name: "Time" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("checkbox", { name: "Action" }));
    expect(onHiddenKeysChange).toHaveBeenLastCalledWith(["action"]);
    expect(screen.queryByRole("columnheader", { name: "Action" })).not.toBeInTheDocument();
    expect(screen.queryByText("spec.create")).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("checkbox", { name: "Action" }));
    expect(screen.getByRole("columnheader", { name: "Action" })).toBeInTheDocument();
    expect(screen.getByText("spec.create")).toBeInTheDocument();
  });

  it("starts with default hidden columns and closes the View menu with Escape", async () => {
    renderTable({ defaultHiddenKeys: ["id"] });
    expect(screen.queryByRole("columnheader", { name: "Id" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "View" }));
    expect(screen.getByRole("checkbox", { name: "Id" })).not.toBeChecked();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByText("Toggle columns")).not.toBeInTheDocument();
  });

  it("closes the View menu on an outside click", async () => {
    renderTable();
    await userEvent.click(screen.getByRole("button", { name: "View" }));
    await userEvent.click(document.body);
    expect(screen.queryByText("Toggle columns")).not.toBeInTheDocument();
  });

  it("offers no View menu when no column is hideable", () => {
    renderTable({ columns: [{ key: "time", title: "Time" }] });
    expect(screen.queryByRole("button", { name: "View" })).not.toBeInTheDocument();
  });

  it("renders the toolbar slot", () => {
    renderTable({ toolbar: <span>filters here</span> });
    expect(screen.getByText("filters here")).toBeInTheDocument();
  });

  it("works inside a parent that filters the rows", async () => {
    render(<Harness />);
    await userEvent.click(screen.getByRole("button", { name: "Go to last page" }));
    expect(screen.getByText("Page 3 of 3")).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("filter"), "Actor 1");
    expect(screen.getByText("Page 2 of 2")).toBeInTheDocument();
    await userEvent.clear(screen.getByLabelText("filter"));
    await userEvent.type(screen.getByLabelText("filter"), "zzz");
    expect(screen.getByText("No results.")).toBeInTheDocument();
    expect(screen.getByText("Page 1 of 1")).toBeInTheDocument();
  });
});
