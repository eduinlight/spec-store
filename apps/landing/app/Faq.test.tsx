import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { Faq } from "./sections/Faq";

const ITEMS = [
  { q: "First question", a: "First answer" },
  { q: "Second question", a: "Second answer" },
  { q: "Third question", a: "Third answer" },
];

const detailsOf = (question: string) =>
  screen.getByText(question).closest("details") as HTMLDetailsElement;

test("the first question is open on load and the others are closed", () => {
  render(<Faq title="Questions" items={ITEMS} />);
  expect(detailsOf("First question").open).toBe(true);
  expect(detailsOf("Second question").open).toBe(false);
  expect(detailsOf("Third question").open).toBe(false);
});

test("opening another question closes the one that was open", async () => {
  render(<Faq title="Questions" items={ITEMS} />);
  const second = detailsOf("Second question");
  second.open = true;
  fireEvent(second, new Event("toggle"));
  await waitFor(() => expect(detailsOf("First question").open).toBe(false));
  expect(second.open).toBe(true);
  expect(detailsOf("Third question").open).toBe(false);
});

test("closing the open question leaves all of them closed", async () => {
  render(<Faq title="Questions" items={ITEMS} />);
  const first = detailsOf("First question");
  first.open = false;
  fireEvent(first, new Event("toggle"));
  await waitFor(() => expect(first.open).toBe(false));
  expect(detailsOf("Second question").open).toBe(false);
});
