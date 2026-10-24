import { cn } from "@workspace/ui/lib/utils";
import { Avatar, Tag } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";
import { type ActivityRow, actionKeyOf, sentenceOf } from "~/lib/activity";
import { useDate } from "~/lib/format";

export interface ActivityItemProps {
  row: ActivityRow;
  rows: ActivityRow[];
  compact?: boolean;
  fresh?: boolean;
}

const PILL_CLASSES: Record<string, string> = {
  ok: "bg-[var(--success-100)] text-[var(--success-700)]",
  denied: "bg-[var(--danger-100)] text-[var(--danger-700)]",
  conflict: "bg-[var(--warning-100)] text-[var(--warning-700)]",
};

type Translate = (key: string, values: Record<string, string>) => string;

export function useSentence(row: ActivityRow, rows: ActivityRow[]): string {
  const { t } = useTranslation();
  const sentence = sentenceOf(row, rows);
  const key = actionKeyOf(row.action);
  const values =
    sentence.key === "denied" && key
      ? { ...sentence.values, action: t(`activity.tried.${key}`) }
      : sentence.values;
  return (t as unknown as Translate)(`activity.${sentence.key}`, values);
}

export function ActivityItem({ row, rows, compact = false, fresh = false }: ActivityItemProps) {
  const { t } = useTranslation();
  const date = useDate();
  const sentence = useSentence(row, rows);
  const result = row.result in PILL_CLASSES ? row.result : "ok";
  return (
    <li
      data-fresh={fresh ? "true" : undefined}
      className={cn(
        "flex gap-3 border-t border-[var(--neutral-200)] transition-colors duration-700",
        compact ? "gap-2.5 py-2" : "items-start px-3 py-3.5",
        fresh && "bg-[var(--primary-50)]",
      )}
    >
      <Avatar name={row.actor} size={compact ? 24 : 32} />
      <div className="flex min-w-0 flex-1 flex-col text-[13px] leading-5">
        <span className="text-[var(--neutral-600)]">
          <b className="font-medium text-[var(--neutral-700)]">{row.actor}</b> {sentence}
        </span>
        <span className="flex flex-wrap items-center gap-2 text-xs text-[var(--neutral-500)]">
          {row.client && <Tag>{t("activity.via", { client: row.client })}</Tag>}
          <time dateTime={row.at}>{date(row.at)}</time>
        </span>
      </div>
      {!compact && (
        <span
          className={cn(
            "h-5 self-start rounded-full px-2 text-xs font-medium leading-5",
            PILL_CLASSES[result],
          )}
        >
          {t(`activity.results.${result as "ok"}`)}
        </span>
      )}
    </li>
  );
}
