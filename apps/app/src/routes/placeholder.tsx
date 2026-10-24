import { Icon } from "@workspace/ui/taptap";
import { useTranslation } from "react-i18next";

export type PlaceholderKey = "members" | "tokens" | "preferences" | "security" | "billing";

interface PlaceholderProps {
  name: PlaceholderKey;
}

export function Placeholder({ name }: PlaceholderProps) {
  const { t } = useTranslation();
  return (
    <section className="mx-auto mt-10 flex max-w-xl flex-col items-center gap-3 rounded bg-white px-8 py-14 text-center">
      <span className="flex size-12 items-center justify-center rounded-full bg-[var(--primary-100)] text-[var(--primary-700)]">
        <Icon name="Time" size={24} />
      </span>
      <span className="text-xs font-medium uppercase tracking-wide text-[var(--primary-700)]">
        {t("placeholder.comingSoon")}
      </span>
      <h1 className="text-2xl font-medium leading-8">{t(`placeholder.${name}.title`)}</h1>
      <p className="text-sm leading-[22px] text-[var(--neutral-600)]">
        {t(`placeholder.${name}.body`)}
      </p>
    </section>
  );
}

export const MembersPlaceholder = () => <Placeholder name="members" />;
export const TokensPlaceholder = () => <Placeholder name="tokens" />;
export const PreferencesPlaceholder = () => <Placeholder name="preferences" />;
export const SecurityPlaceholder = () => <Placeholder name="security" />;
export const BillingPlaceholder = () => <Placeholder name="billing" />;
