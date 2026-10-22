export type Language = "en" | "es";

export const LANGUAGES: Language[] = ["en", "es"];
export const LANGUAGE_COOKIE = "spec_store_lang";
export const ONE_YEAR_SECONDS = 31_536_000;

function supported(value: string | undefined): Language | undefined {
  const base = value?.toLowerCase().split("-")[0];
  return LANGUAGES.find((language) => language === base);
}

export function readLanguageCookie(cookie: string): Language | undefined {
  const value = cookie
    .split(";")
    .map((pair) => pair.trim().split("="))
    .find(([name]) => name === LANGUAGE_COOKIE)?.[1];
  return supported(value);
}

export function detectLanguage({
  cookie,
  languages,
}: {
  cookie: string;
  languages: readonly string[];
}): Language {
  return readLanguageCookie(cookie) ?? languages.map(supported).find(Boolean) ?? "en";
}

export function cookieDomain(hostname: string): string | undefined {
  if (hostname === "localhost" || /^[\d.]+$/.test(hostname) || hostname.includes(":"))
    return undefined;
  const labels = hostname.split(".");
  return labels.length >= 2 ? labels.slice(-2).join(".") : undefined;
}

export function languageCookie(language: Language, hostname: string, protocol: string): string {
  const parts = [
    `${LANGUAGE_COOKIE}=${language}`,
    "Path=/",
    `Max-Age=${ONE_YEAR_SECONDS}`,
    "SameSite=Lax",
  ];
  const domain = cookieDomain(hostname);
  if (domain) parts.push(`Domain=${domain}`);
  if (protocol === "https:") parts.push("Secure");
  return parts.join("; ");
}

export function rememberLanguage(language: Language) {
  document.cookie = languageCookie(language, window.location.hostname, window.location.protocol);
}

export function browserLanguage(): Language {
  return detectLanguage({
    cookie: document.cookie,
    languages: navigator.languages ?? [navigator.language],
  });
}
