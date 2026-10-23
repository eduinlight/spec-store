import i18next, { type i18n } from "i18next";
import { initReactI18next } from "react-i18next";
import { en } from "../i18n/en";
import { es } from "../i18n/es";
import { browserLanguage, type Language, rememberLanguage } from "./language";

export const resources = {
  en: { translation: en },
  es: { translation: es },
};

export function createI18n(language: Language = browserLanguage()): i18n {
  const instance = i18next.createInstance();
  instance.use(initReactI18next).init({
    resources,
    lng: language,
    fallbackLng: "en",
    supportedLngs: ["en", "es"],
    interpolation: { escapeValue: false },
    initAsync: false,
  });
  return instance;
}

export async function switchLanguage(instance: i18n, language: Language) {
  rememberLanguage(language);
  await instance.changeLanguage(language);
  document.documentElement.lang = language;
}
