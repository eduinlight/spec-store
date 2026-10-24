export interface SpecProgressProps {
  done: number;
  total: number;
  label: string;
}

export function SpecProgress({ done, total, label }: SpecProgressProps) {
  const percent = total === 0 ? 0 : Math.round((done / total) * 100);
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={total}
      aria-valuenow={done}
      className="h-1.5 overflow-hidden rounded-full bg-[var(--neutral-200)]"
    >
      <div className="h-full bg-[var(--primary-600)]" style={{ width: `${percent}%` }} />
    </div>
  );
}
