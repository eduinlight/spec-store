import { Icon, Tooltip } from "@workspace/ui/taptap";

export interface InfoTipProps {
  label: string;
  text: string;
}

export function InfoTip({ label, text }: InfoTipProps) {
  return (
    <Tooltip title={text}>
      <button
        type="button"
        aria-label={label}
        className="flex cursor-help border-0 bg-transparent p-0 text-[var(--neutral-400)]"
      >
        <Icon name="Info" size={12} />
      </button>
    </Tooltip>
  );
}
