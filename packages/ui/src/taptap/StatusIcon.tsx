export type StatusKind = "info" | "success" | "warning" | "error";

const STATUS_ICON: Record<StatusKind, [string, string]> = {
  info: ["var(--info-600)", "M8 4.5v1M8 7v4.5"],
  success: ["var(--success-600)", "M5 8.2 7.2 10.3 11 6"],
  warning: ["var(--warning-600)", "M8 4.5v4.5M8 10.6v1"],
  error: ["var(--danger-600)", "M5.8 5.8l4.4 4.4m0-4.4-4.4 4.4"],
};

export interface StatusIconProps {
  status?: StatusKind;
  size?: number;
}

export function StatusIcon({ status = "info", size = 18 }: StatusIconProps) {
  const [color, path] = STATUS_ICON[status] ?? STATUS_ICON.info;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      aria-hidden="true"
      focusable="false"
      style={{ flexShrink: 0 }}
    >
      <circle cx="8" cy="8" r="7.5" fill={color} />
      <path d={path} stroke="#fff" strokeWidth="1.4" fill="none" strokeLinecap="round" />
    </svg>
  );
}

export interface CloseXProps {
  size?: number;
}

export function CloseX({ size = 14 }: CloseXProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 14 14" aria-hidden="true" focusable="false">
      <path
        fill="currentColor"
        transform="translate(.754 .754)"
        d="M5.421 6.246 0 11.667l.825.825 5.421-5.421 5.421 5.421.825-.825-5.421-5.421L12.492.825 11.667 0 6.246 5.421.825 0 0 .825z"
      />
    </svg>
  );
}
