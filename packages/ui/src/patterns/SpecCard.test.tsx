import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SpecCard, type SpecCardProps } from "./SpecCard";

const documents = [
  { key: "S", title: "Spec", written: true },
  { key: "P", title: "Plan", written: true },
  { key: "T", title: "Tasks", written: false },
  { key: "R", title: "Research", written: false },
  { key: "V", title: "Validation", written: false },
];

const base: SpecCardProps = {
  number: "004",
  title: "Diesel ORM migration",
  status: "currentVersion",
  statusLabel: "Implementing",
  createdLabel: "Oct 12",
  summary: "Move the persistence layer to Diesel.",
  slug: "004-diesel-orm-migration",
  documents,
  requirementCount: 18,
  updatedLabel: "2 min ago",
  contributors: ["Ada Lovelace", "Grace Hopper"],
  tasksDone: 12,
  tasksTotal: 20,
  accent: "active",
};

describe("SpecCard", () => {
  it("is a link named by its number and title", () => {
    render(<SpecCard {...base} href="/specs/004" />);
    const card = screen.getByRole("link", { name: "004 Diesel ORM migration" });
    expect(card).toHaveAttribute("href", "/specs/004");
  });

  it("is a button that calls onClick when there is no href", async () => {
    const onClick = vi.fn();
    render(<SpecCard {...base} onClick={onClick} />);
    await userEvent.click(screen.getByRole("button", { name: "004 Diesel ORM migration" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("opens with the keyboard", async () => {
    const onClick = vi.fn();
    render(<SpecCard {...base} onClick={onClick} />);
    await userEvent.tab();
    expect(screen.getByRole("button", { name: "004 Diesel ORM migration" })).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("shows the number, created label, status, summary, slug, requirements and update", () => {
    render(<SpecCard {...base} href="/x" />);
    expect(screen.getByText("Oct 12")).toBeInTheDocument();
    expect(screen.getByText("Implementing")).toBeInTheDocument();
    expect(screen.getByText("Move the persistence layer to Diesel.")).toBeInTheDocument();
    expect(screen.getByText("004-diesel-orm-migration")).toBeInTheDocument();
    expect(screen.getByText("18 requirements")).toBeInTheDocument();
    expect(screen.getByText("Updated 2 min ago")).toBeInTheDocument();
  });

  it("renders five document chips, filled only when written", () => {
    render(<SpecCard {...base} href="/x" />);
    const spec = screen.getByRole("img", { name: "Spec · written" });
    const tasks = screen.getByRole("img", { name: "Tasks · not written yet" });
    expect(spec).toHaveTextContent("S");
    expect(spec.style.background).toBe("var(--primary-100)");
    expect(tasks).toHaveTextContent("T");
    expect(tasks.style.background).toBe("transparent");
    expect(tasks.style.boxShadow).toBe("inset 0 0 0 1px var(--neutral-300)");
    expect(screen.getAllByRole("img", { name: /written/ })).toHaveLength(5);
  });

  it("shows the document tooltip on hover", async () => {
    render(<SpecCard {...base} href="/x" />);
    await userEvent.hover(screen.getByRole("img", { name: "Plan · written" }));
    expect(screen.getByRole("tooltip")).toHaveTextContent("Plan · written");
  });

  it("shows the live agents pill only when agents are live", () => {
    const { rerender } = render(<SpecCard {...base} href="/x" />);
    expect(screen.queryByText(/agents? live/)).not.toBeInTheDocument();
    rerender(<SpecCard {...base} href="/x" liveAgents={3} />);
    expect(screen.getByText("3 agents live")).toBeInTheDocument();
    rerender(<SpecCard {...base} href="/x" liveAgents={1} />);
    expect(screen.getByText("1 agent live")).toBeInTheDocument();
  });

  it("shows the open questions pill with a tooltip", async () => {
    const { rerender } = render(<SpecCard {...base} href="/x" />);
    expect(screen.queryByText(/open question/)).not.toBeInTheDocument();
    rerender(
      <SpecCard {...base} href="/x" openQuestions={2} openQuestionsTip="Answer before approval" />,
    );
    const pill = screen.getByText("2 open questions");
    expect(pill.style.cursor).toBe("help");
    await userEvent.hover(pill);
    expect(screen.getByRole("tooltip")).toHaveTextContent("Answer before approval");
  });

  it("lists the contributors", () => {
    render(<SpecCard {...base} href="/x" />);
    expect(screen.getByRole("img", { name: "Ada Lovelace" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Grace Hopper" })).toBeInTheDocument();
  });

  it("shows task progress as a bar and text", () => {
    render(<SpecCard {...base} href="/x" />);
    expect(screen.getByRole("progressbar", { name: "Task progress" })).toHaveAttribute(
      "aria-valuenow",
      "60",
    );
    expect(screen.getByText("12 / 20 tasks")).toBeInTheDocument();
  });

  it("says there are no tasks yet when the total is zero", () => {
    render(<SpecCard {...base} href="/x" tasksDone={0} tasksTotal={0} />);
    expect(screen.getByText("No tasks yet")).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "0");
  });

  it("draws the accent bar and ring by accent, and lifts on hover", async () => {
    const { container, rerender } = render(<SpecCard {...base} href="/x" accent="active" />);
    const card = screen.getByRole("link");
    expect(card.style.boxShadow).toBe("inset 0 0 0 1px var(--primary-300)");
    const bar = () => container.querySelector("span[aria-hidden='true']") as HTMLElement;
    expect(bar().style.background).toBe("var(--primary-600)");
    expect(bar().style.width).toBe("3px");
    await userEvent.hover(card);
    expect(card.style.boxShadow).toBe("var(--elevation-2)");
    rerender(<SpecCard {...base} href="/x" accent="draft" />);
    expect(bar().style.background).toBe("var(--warning-600)");
  });

  it("mutes an archived card", () => {
    render(<SpecCard {...base} href="/x" archived />);
    expect(screen.getByText("004").style.color).toBe("var(--neutral-400)");
  });
});

test("uses the labels it is given for every count and state", () => {
  render(
    <SpecCard
      number="004"
      title="Diesel"
      status="approved"
      createdLabel="oct"
      summary="Resumen"
      slug="004-diesel"
      documents={[{ key: "S", title: "Spec", written: false }]}
      requirementCount={2}
      updatedLabel="hoy"
      contributors={[]}
      tasksDone={1}
      tasksTotal={3}
      openQuestions={2}
      labels={{
        openQuestions: (count) => `${count} preguntas abiertas`,
        requirements: (count) => `${count} requisitos`,
        tasks: (done, total) => `${done} / ${total} tareas`,
        notWritten: "sin escribir",
      }}
    />,
  );
  expect(screen.getByText("2 preguntas abiertas")).toBeInTheDocument();
  expect(screen.getByText("2 requisitos")).toBeInTheDocument();
  expect(screen.getByText("1 / 3 tareas")).toBeInTheDocument();
});
