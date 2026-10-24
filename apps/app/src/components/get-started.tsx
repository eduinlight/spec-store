import { cn } from "@workspace/ui/lib/utils";
import { Button, Icon, message } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";

export interface GetStartedProps {
  command: string;
  agentCalled: boolean;
  specWritten: boolean;
}

interface Step {
  key: "create" | "connect" | "call" | "spec";
  done: boolean;
}

export function GetStarted({ command, agentCalled, specWritten }: GetStartedProps) {
  const { t } = useTranslation();
  const steps: Step[] = [
    { key: "create", done: true },
    { key: "connect", done: agentCalled || specWritten },
    { key: "call", done: agentCalled || specWritten },
    { key: "spec", done: specWritten },
  ];
  const done = steps.filter((step) => step.done).length;
  const copy = async () => {
    await navigator.clipboard.writeText(command);
    message.success(t("getStarted.connect.copied"));
  };
  return (
    <section className="flex flex-col gap-1 rounded bg-white p-6">
      <div className="mb-3 flex items-baseline gap-3">
        <h2 className="m-0 flex-1 text-lg font-medium leading-[26px]">{t("getStarted.title")}</h2>
        <span className="text-[13px] text-[var(--neutral-500)]">
          {t("getStarted.progress", { done, total: steps.length })}
        </span>
      </div>
      <ol aria-label={t("getStarted.title")} className="m-0 flex list-none flex-col p-0">
        {steps.map((step, index) => (
          <li
            key={step.key}
            aria-label={t(`getStarted.${step.key}.title`)}
            className="grid items-start gap-3.5 border-t border-[var(--neutral-200)] py-3.5 [grid-template-columns:28px_minmax(0,1fr)_auto]"
          >
            <span
              className={cn(
                "flex size-6 items-center justify-center rounded-full text-xs font-medium",
                step.done
                  ? "bg-[var(--success-600)] text-white"
                  : "bg-[var(--neutral-100)] text-[var(--neutral-600)] shadow-[inset_0_0_0_1px_var(--neutral-300)]",
              )}
            >
              {step.done ? <Icon name="Check" size={14} /> : index + 1}
            </span>
            <div className="flex flex-col gap-0.5">
              <span
                className={cn(
                  "text-sm font-medium leading-[22px]",
                  step.done ? "text-[var(--neutral-500)]" : "text-[var(--neutral-700)]",
                )}
              >
                {t(`getStarted.${step.key}.title`)}
              </span>
              <span className="text-[13px] leading-5 text-[var(--neutral-500)]">
                {t(`getStarted.${step.key}.description`)}
              </span>
              {step.key === "connect" && (
                <code className="mt-2 block whitespace-pre-wrap break-all rounded bg-[var(--neutral-700)] px-3 py-2.5 font-mono text-xs leading-5 text-[#E1E1E1]">
                  {command}
                </code>
              )}
            </div>
            <span className="flex items-center">
              {step.key === "connect" && (
                <Button variant="outline" size="small" onClick={() => void copy()}>
                  {t("getStarted.connect.copy")}
                </Button>
              )}
              {step.done && (
                <span className="text-xs text-[var(--success-700)]">{t("getStarted.done")}</span>
              )}
            </span>
          </li>
        ))}
      </ol>
    </section>
  );
}
