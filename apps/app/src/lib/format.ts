import { useTranslation } from "react-i18next";
import { formatDate, formatMonth } from "./dates";
import type { Language } from "./language";

export function useDate() {
  const { i18n } = useTranslation();
  const language = (i18n.language === "es" ? "es" : "en") as Language;
  return (iso: string) => formatDate(iso, language);
}

export function useMonth() {
  const { i18n } = useTranslation();
  const language = (i18n.language === "es" ? "es" : "en") as Language;
  return (iso: string) => formatMonth(iso, language);
}

export const specNumber = (number: number): string => String(number).padStart(3, "0");

export function branch(number: number, slug: string): string {
  return `${String(number).padStart(3, "0")}-${slug}`;
}

export const SPEC_KINDS = ["spec", "plan", "tasks", "research", "validation"] as const;
export type SpecKind = (typeof SPEC_KINDS)[number];
