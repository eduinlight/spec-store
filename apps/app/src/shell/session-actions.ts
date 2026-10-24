import type { i18n } from "i18next";
import { api } from "~/api/client";
import { switchLanguage } from "~/lib/i18n";
import type { Language } from "~/lib/language";

export async function signOut() {
  const result = await api.POST("/auth/logout");
  if (result.data?.logout_url) {
    window.location.assign(result.data.logout_url);
  } else {
    window.location.assign("/");
  }
}

export async function changeLanguage(instance: i18n, language: Language) {
  await switchLanguage(instance, language);
  await api.PUT("/api/v1/me/language", { body: { language } });
}
