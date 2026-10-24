import type { CSSProperties } from "react";

export const CHART_COLORS = [
  "var(--chart-1)",
  "var(--chart-2)",
  "var(--chart-3)",
  "var(--chart-4)",
  "var(--chart-5)",
  "var(--chart-6)",
];

const AXIS_TEXT: CSSProperties = {
  font: "400 12px/18px var(--font-sans)",
  fill: "var(--neutral-500)",
};

export interface ChartLegendItem {
  label: string;
  color?: string;
}

export interface ChartLegendProps {
  items?: ChartLegendItem[];
  shape?: "square" | "dot" | "line";
  direction?: "horizontal" | "vertical";
  style?: CSSProperties;
}

export function ChartLegend({
  items = [],
  shape = "square",
  direction = "horizontal",
  style,
}: ChartLegendProps) {
  return (
    <ul
      aria-label="Legend"
      style={{
        display: "flex",
        flexDirection: direction === "vertical" ? "column" : "row",
        gap: direction === "vertical" ? 12 : 24,
        justifyContent: "center",
        margin: 0,
        padding: 0,
        listStyle: "none",
        font: "400 12px/18px var(--font-sans)",
        color: "var(--neutral-600)",
        ...style,
      }}
    >
      {items.map((item, index) => (
        <li key={item.label} style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
          <span
            data-swatch=""
            aria-hidden="true"
            style={{
              width: shape === "line" ? 12 : 8,
              height: shape === "line" ? 2 : 8,
              borderRadius: shape === "dot" ? "50%" : 0,
              background: item.color ?? CHART_COLORS[index % CHART_COLORS.length],
            }}
          />
          {item.label}
        </li>
      ))}
    </ul>
  );
}

export interface BarSeries {
  name: string;
  data: number[];
  color?: string;
}

export interface BarChartProps {
  labels?: string[];
  series?: BarSeries[];
  stacked?: boolean;
  height?: number;
  max?: number;
  label?: string;
  style?: CSSProperties;
}

const WIDTH = 1000;
const PAD = { l: 56, r: 16, t: 16, b: 32 };
const TICKS = 5;

export function BarChart({
  labels = [],
  series = [],
  stacked = true,
  height = 320,
  max,
  label = "Bar chart",
  style,
}: BarChartProps) {
  const valueAt = (item: BarSeries, index: number) => item.data[index] || 0;
  const totals = labels.map((_, index) =>
    stacked
      ? series.reduce((sum, item) => sum + valueAt(item, index), 0)
      : Math.max(0, ...series.map((item) => valueAt(item, index))),
  );
  const top = max || Math.ceil(Math.max(...totals, 1) / 1000) * 1000;
  const barSlot = labels.length > 0 ? (WIDTH - PAD.l - PAD.r) / labels.length : 0;
  const scale = (value: number) => ((height - PAD.t - PAD.b) * value) / top;
  const colorOf = (item: BarSeries, index: number) =>
    item.color ?? CHART_COLORS[index % CHART_COLORS.length];
  const summary = `${label}, ${stacked ? "stacked" : "grouped"}. ${labels.length} categories: ${labels.join(", ")}. ${series
    .map((item) => `${item.name} total ${item.data.reduce((sum, value) => sum + (value || 0), 0)}`)
    .join("; ")}.`;
  return (
    <div style={style}>
      <svg
        role="img"
        aria-label={summary}
        viewBox={`0 0 ${WIDTH} ${height}`}
        width="100%"
        height={height}
        preserveAspectRatio="none"
        style={{ display: "block", overflow: "visible" }}
      >
        <g>
          {Array.from({ length: TICKS + 1 }, (_, index) => {
            const y = PAD.t + (height - PAD.t - PAD.b) * (1 - index / TICKS);
            return (
              <g key={y}>
                <line
                  x1={PAD.l}
                  x2={WIDTH - PAD.r}
                  y1={y}
                  y2={y}
                  stroke="var(--neutral-200)"
                  strokeDasharray={index ? "4 4" : "0"}
                />
                <text x={PAD.l - 8} y={y + 4} textAnchor="end" style={AXIS_TEXT}>
                  {Math.round((top * index) / TICKS)}
                </text>
              </g>
            );
          })}
        </g>
        {labels.map((name, index) => {
          let accumulated = 0;
          const x = PAD.l + barSlot * index + barSlot / 2;
          return (
            <g key={name}>
              {series.map((item, seriesIndex) => {
                const value = valueAt(item, index);
                const barHeight = scale(value);
                const barX = stacked ? x - 14 : x - 14 * series.length + seriesIndex * 28;
                const barY = height - PAD.b - (stacked ? accumulated + barHeight : barHeight);
                accumulated += stacked ? barHeight : 0;
                return (
                  <rect
                    key={item.name}
                    x={barX}
                    y={barY}
                    width={28}
                    height={barHeight}
                    fill={colorOf(item, seriesIndex)}
                  >
                    <title>{`${name}, ${item.name}: ${value}`}</title>
                  </rect>
                );
              })}
              <text x={x} y={height - PAD.b + 20} textAnchor="middle" style={AXIS_TEXT}>
                {name}
              </text>
            </g>
          );
        })}
      </svg>
      <ChartLegend
        items={series.map((item, index) => ({ label: item.name, color: colorOf(item, index) }))}
        style={{ marginTop: 8 }}
      />
    </div>
  );
}
