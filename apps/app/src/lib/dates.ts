import dayjs from "dayjs";
import "dayjs/locale/es";
import localizedFormat from "dayjs/plugin/localizedFormat";
import relativeTime from "dayjs/plugin/relativeTime";
import type { Language } from "./language";

dayjs.extend(relativeTime);
dayjs.extend(localizedFormat);

export const RELATIVE_DAYS = 7;

export function formatDate(iso: string, language: Language, now: Date = new Date()): string {
  const moment = dayjs(iso).locale(language);
  const reference = dayjs(now);
  if (reference.diff(moment, "day", true) < RELATIVE_DAYS) {
    return moment.from(reference);
  }
  return moment.format("LLL");
}

export function formatMonth(iso: string, language: Language): string {
  return dayjs(iso).locale(language).format("MMM YYYY");
}
