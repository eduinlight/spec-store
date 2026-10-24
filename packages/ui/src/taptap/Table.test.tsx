import { fireEvent, render, screen, within } from "@testing-library/react";
import { Table, type TableColumn } from "./Table";

interface Row {
  id: number;
  name: string;
  total: number;
}

const columns: TableColumn<Row>[] = [
  { key: "name", title: "Name", width: 200 },
  { key: "total", title: "Total", align: "right", render: (value) => `$${value}` },
];

const data: Row[] = [
  { id: 1, name: "Alpha", total: 10 },
  { id: 2, name: "Beta", total: 20 },
];

describe("Table", () => {
  it("renders headers, rows and custom cells", () => {
    render(<Table columns={columns} data={data} />);
    expect(screen.getByRole("columnheader", { name: "Name" })).toHaveStyle({ width: "200px" });
    expect(screen.getByRole("columnheader", { name: "Total" })).toHaveStyle({ textAlign: "right" });
    expect(screen.getAllByRole("row")).toHaveLength(3);
    expect(screen.getByText("$20")).toBeInTheDocument();
  });

  it("shows the empty text without rows", () => {
    render(<Table columns={columns} data={[]} emptyText="Nothing here" />);
    expect(screen.getByText("Nothing here")).toBeInTheDocument();
  });

  it("highlights the hovered row", () => {
    render(<Table columns={columns} data={data} />);
    const row = screen.getByText("Alpha").closest("tr") as HTMLElement;
    const background = () => row.getAttribute("style");
    expect(background()).toContain("background: rgb(255, 255, 255)");
    fireEvent.mouseEnter(row);
    expect(background()).toContain("background: var(--neutral-50)");
    fireEvent.mouseLeave(row);
    expect(background()).toContain("background: rgb(255, 255, 255)");
  });

  it("selects rows and all rows", () => {
    const onSelect = vi.fn();
    render(<Table columns={columns} data={data} selectable selected={[1]} onSelect={onSelect} />);
    const rows = screen.getAllByRole("checkbox", { name: "Select row" });
    fireEvent.click(rows[1] as HTMLElement);
    expect(onSelect).toHaveBeenLastCalledWith([1, 2]);
    fireEvent.click(rows[0] as HTMLElement);
    expect(onSelect).toHaveBeenLastCalledWith([]);
    fireEvent.click(screen.getByRole("checkbox", { name: "Select all rows" }));
    expect(onSelect).toHaveBeenLastCalledWith([1, 2]);
  });

  it("expands a row", () => {
    render(<Table columns={columns} data={data} expandable={(row) => `Details ${row.name}`} />);
    fireEvent.click(screen.getAllByRole("button", { name: "Expand row" })[0] as HTMLElement);
    expect(screen.getByText("Details Alpha")).toBeInTheDocument();
    expect(
      within(screen.getByText("Alpha").closest("tr") as HTMLElement).getByRole("button"),
    ).toHaveAttribute("aria-expanded", "true");
  });
});
