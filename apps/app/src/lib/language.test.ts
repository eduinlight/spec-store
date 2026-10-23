import { cookieDomain, detectLanguage, languageCookie, readLanguageCookie } from "./language";

test("the first visit follows the browser languages with an English fallback", () => {
  expect(detectLanguage({ cookie: "", languages: ["es-AR", "en"] })).toBe("es");
  expect(detectLanguage({ cookie: "", languages: ["en-GB"] })).toBe("en");
  expect(detectLanguage({ cookie: "", languages: ["fr", "de"] })).toBe("en");
  expect(detectLanguage({ cookie: "", languages: [] })).toBe("en");
});

test("a remembered choice wins over the browser", () => {
  expect(detectLanguage({ cookie: "x=1; spec_store_lang=es", languages: ["en"] })).toBe("es");
  expect(detectLanguage({ cookie: "spec_store_lang=en", languages: ["es"] })).toBe("en");
  expect(detectLanguage({ cookie: "spec_store_lang=fr", languages: ["es"] })).toBe("es");
  expect(readLanguageCookie("a=b; spec_store_lang=es")).toBe("es");
});

test("the language cookie is shared by the landing and the app for a year", () => {
  const cookie = languageCookie("es", "app-spec-store.eduindev.com", "https:");
  expect(cookie).toContain("spec_store_lang=es");
  expect(cookie).toContain("Domain=eduindev.com");
  expect(cookie).toContain("Max-Age=31536000");
  expect(cookie).toContain("Path=/");
  expect(cookie).toContain("SameSite=Lax");
  expect(cookie).toContain("Secure");
  expect(cookieDomain("staging-app-spec-store.eduindev.com")).toBe("eduindev.com");
  expect(cookieDomain("localhost")).toBeUndefined();
  const local = languageCookie("en", "localhost", "http:");
  expect(local).not.toContain("Domain");
  expect(local).not.toContain("Secure");
});
