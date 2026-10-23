import { formatDate } from "./dates";

const now = new Date("2026-10-22T12:00:00.000Z");

test("times under seven days are relative in both languages", () => {
  expect(formatDate("2026-10-22T09:00:00.000Z", "en", now)).toBe("3 hours ago");
  expect(formatDate("2026-10-22T09:00:00.000Z", "es", now)).toBe("hace 3 horas");
  expect(formatDate("2026-10-16T12:00:01.000Z", "en", now)).toBe("6 days ago");
});

test("times from seven days on are absolute in the viewer's time zone", () => {
  expect(formatDate("2026-10-01T18:30:00.000Z", "en", now)).toBe("October 1, 2026 2:30 PM");
  expect(formatDate("2026-10-01T18:30:00.000Z", "es", now)).toBe("1 de octubre de 2026 14:30");
  expect(formatDate("2026-10-15T12:00:00.000Z", "en", now)).toBe("October 15, 2026 8:00 AM");
});
