import { render, screen } from "@testing-library/react";
import { BarChart, CHART_COLORS, ChartLegend } from "./BarChart";

const labels = ["Jan", "Feb"];
const series = [
  { name: "Reads", data: [100, 200] },
  { name: "Writes", data: [50, 25], color: "red" },
];

describe("CHART_COLORS", () => {
  it("offers six chart tokens", () => {
    expect(CHART_COLORS).toHaveLength(6);
    expect(CHART_COLORS[0]).toBe("var(--chart-1)");
  });
});

describe("ChartLegend", () => {
  it("lists the series with colour swatches", () => {
    const { container } = render(
      <ChartLegend items={[{ label: "A" }, { label: "B", color: "blue" }]} />,
    );
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    const swatches = Array.from(container.querySelectorAll<HTMLElement>("[data-swatch]"));
    expect(swatches[0]?.getAttribute("style")).toContain("background: var(--chart-1)");
    expect(swatches[1]?.getAttribute("style")).toContain("background: blue");
  });

  it("draws dots, lines and vertical lists", () => {
    const { container } = render(
      <ChartLegend shape="dot" direction="vertical" items={[{ label: "A" }]} />,
    );
    expect(container.querySelector("[data-swatch]")).toHaveStyle({ borderRadius: "50%" });
    expect(screen.getByRole("list")).toHaveStyle({ flexDirection: "column" });
  });
});

describe("BarChart", () => {
  it("is an image with a label summarising the series", () => {
    render(<BarChart labels={labels} series={series} />);
    const chart = screen.getByRole("img");
    const label = chart.getAttribute("aria-label");
    expect(label).toContain("stacked");
    expect(label).toContain("Jan, Feb");
    expect(label).toContain("Reads total 300");
    expect(label).toContain("Writes total 75");
  });

  it("gives every bar a title with its value", () => {
    const { container } = render(<BarChart labels={labels} series={series} />);
    const titles = Array.from(container.querySelectorAll("rect > title")).map(
      (node) => node.textContent,
    );
    expect(titles).toEqual([
      "Jan, Reads: 100",
      "Jan, Writes: 50",
      "Feb, Reads: 200",
      "Feb, Writes: 25",
    ]);
  });

  it("stacks bars on the same column and groups them otherwise", () => {
    const { container, rerender } = render(<BarChart labels={labels} series={series} />);
    const xs = () =>
      Array.from(container.querySelectorAll("rect")).map((node) => node.getAttribute("x"));
    const stacked = xs();
    expect(stacked[0]).toBe(stacked[1]);
    rerender(<BarChart labels={labels} series={series} stacked={false} />);
    const grouped = xs();
    expect(grouped[0]).not.toBe(grouped[1]);
  });

  it("colours bars from the series or the palette and draws the legend", () => {
    const { container } = render(<BarChart labels={labels} series={series} height={240} />);
    const rects = container.querySelectorAll("rect");
    expect(rects[0]).toHaveAttribute("fill", "var(--chart-1)");
    expect(rects[1]).toHaveAttribute("fill", "red");
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    expect(screen.getByRole("img")).toHaveAttribute("height", "240");
  });

  it("renders without data", () => {
    render(<BarChart />);
    expect(screen.getByRole("img")).toBeInTheDocument();
  });
});
