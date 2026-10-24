import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Combobox, type ComboboxItem } from "./Combobox";

const items: ComboboxItem[] = [
  { value: "acme", label: "Acme Labs", initial: "A", meta: "Owner" },
  { value: "globex", label: "Globex", initial: "G", meta: "Writer" },
  { value: "initech", label: "Initech", initial: "I", meta: "Reader" },
];

const trigger = () => screen.getByRole("button", { name: /Acme Labs|Select/ });

const renderCombobox = (props: Partial<React.ComponentProps<typeof Combobox>> = {}) =>
  render(
    <Combobox
      items={items}
      defaultValue="acme"
      groupLabel="Organizations"
      searchPlaceholder="Search organization…"
      emptyText="No organization found."
      {...props}
    />,
  );

describe("Combobox", () => {
  it("renders a closed trigger with the current item", () => {
    renderCombobox();
    expect(trigger()).toHaveAttribute("aria-haspopup", "listbox");
    expect(trigger()).toHaveAttribute("aria-expanded", "false");
    expect(trigger()).toHaveTextContent("Acme Labs");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("shows the placeholder when nothing is selected", () => {
    renderCombobox({ defaultValue: undefined, placeholder: "Pick one" });
    expect(screen.getByRole("button", { name: "Pick one" })).toBeInTheDocument();
  });

  it("opens with the search field focused and the current item selected", async () => {
    renderCombobox();
    await userEvent.click(trigger());
    expect(trigger()).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByPlaceholderText("Search organization…")).toHaveFocus();
    expect(screen.getByText("Organizations")).toBeInTheDocument();
    expect(screen.getByRole("listbox", { name: "Organizations" })).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(3);
    expect(screen.getByRole("option", { name: /Acme Labs/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("option", { name: /Globex/ })).toHaveAttribute(
      "aria-selected",
      "false",
    );
  });

  it("filters case-insensitively", async () => {
    renderCombobox();
    await userEvent.click(trigger());
    await userEvent.type(screen.getByPlaceholderText("Search organization…"), "GLOB");
    expect(screen.getAllByRole("option")).toHaveLength(1);
    expect(screen.getByRole("option", { name: /Globex/ })).toBeInTheDocument();
  });

  it("filters by description", async () => {
    renderCombobox({
      items: [
        { value: "a", label: "alpha", description: "Spec 004" },
        { value: "b", label: "beta", description: "Spec 005" },
      ],
      defaultValue: "a",
    });
    await userEvent.click(screen.getByRole("button", { name: /alpha/ }));
    await userEvent.type(screen.getByPlaceholderText("Search organization…"), "005");
    expect(screen.getAllByRole("option")).toHaveLength(1);
    expect(screen.getByRole("option", { name: /beta/ })).toBeInTheDocument();
  });

  it("shows the empty text when nothing matches", async () => {
    renderCombobox();
    await userEvent.click(trigger());
    await userEvent.type(screen.getByPlaceholderText("Search organization…"), "zzz");
    expect(screen.queryAllByRole("option")).toHaveLength(0);
    expect(screen.getByText("No organization found.")).toBeInTheDocument();
  });

  it("picks an item by click, closes and reports the value", async () => {
    const onChange = vi.fn();
    renderCombobox({ onChange });
    await userEvent.click(trigger());
    await userEvent.click(screen.getByRole("option", { name: /Initech/ }));
    expect(onChange).toHaveBeenCalledWith("initech");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Initech/ })).toHaveFocus();
  });

  it("does not change the shown value when controlled", async () => {
    const onChange = vi.fn();
    renderCombobox({ value: "acme", defaultValue: undefined, onChange });
    await userEvent.click(trigger());
    await userEvent.click(screen.getByRole("option", { name: /Globex/ }));
    expect(onChange).toHaveBeenCalledWith("globex");
    expect(trigger()).toHaveTextContent("Acme Labs");
  });

  it("moves the highlight with the arrow keys and picks with Enter", async () => {
    const onChange = vi.fn();
    renderCombobox({ onChange });
    await userEvent.click(trigger());
    const search = screen.getByPlaceholderText("Search organization…");
    const options = screen.getAllByRole("option");
    expect(search).toHaveAttribute("aria-activedescendant", options[0]?.id);
    await userEvent.keyboard("{ArrowDown}");
    expect(search).toHaveAttribute("aria-activedescendant", options[1]?.id);
    await userEvent.keyboard("{ArrowDown}{ArrowDown}");
    expect(search).toHaveAttribute("aria-activedescendant", options[2]?.id);
    await userEvent.keyboard("{ArrowUp}");
    expect(search).toHaveAttribute("aria-activedescendant", options[1]?.id);
    await userEvent.keyboard("{Enter}");
    expect(onChange).toHaveBeenCalledWith("globex");
  });

  it("jumps to the first and last items with Home and End", async () => {
    renderCombobox();
    await userEvent.click(trigger());
    const search = screen.getByPlaceholderText("Search organization…");
    const options = screen.getAllByRole("option");
    await userEvent.keyboard("{End}");
    expect(search).toHaveAttribute("aria-activedescendant", options[2]?.id);
    await userEvent.keyboard("{Home}");
    expect(search).toHaveAttribute("aria-activedescendant", options[0]?.id);
  });

  it("opens from the trigger with ArrowDown", async () => {
    renderCombobox();
    trigger().focus();
    await userEvent.keyboard("{ArrowDown}");
    expect(screen.getByRole("listbox")).toBeInTheDocument();
  });

  it("closes with Escape and returns focus to the trigger", async () => {
    renderCombobox();
    await userEvent.click(trigger());
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(trigger()).toHaveFocus();
  });

  it("closes on an outside click and clears the query", async () => {
    render(
      <>
        <button type="button">outside</button>
        <Combobox items={items} defaultValue="acme" groupLabel="Organizations" />
      </>,
    );
    await userEvent.click(screen.getByRole("button", { name: /Acme Labs/ }));
    await userEvent.type(screen.getByRole("textbox"), "glo");
    await userEvent.click(screen.getByRole("button", { name: "outside" }));
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Acme Labs/ }));
    expect(screen.getByRole("textbox")).toHaveValue("");
    expect(screen.getAllByRole("option")).toHaveLength(3);
  });

  it("runs footer actions and closes the panel", async () => {
    const onCreate = vi.fn();
    renderCombobox({
      actions: [
        { key: "create", label: "Create organization", icon: "PlusSmall", onSelect: onCreate },
      ],
    });
    await userEvent.click(trigger());
    await userEvent.click(screen.getByRole("button", { name: "Create organization" }));
    expect(onCreate).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("shows the meta badge in the trigger and the project style for mono lists", async () => {
    renderCombobox({ showMetaInTrigger: true, mono: true });
    expect(trigger()).toHaveTextContent("Owner");
    expect(trigger()).toHaveStyle({ height: "36px" });
    await userEvent.click(trigger());
    expect(screen.getAllByRole("option")).toHaveLength(3);
  });
});
