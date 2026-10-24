import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderShell } from "~/test/shell";
import { ConfirmProvider, useConfirm } from "./confirm";

function Harness({ onResult }: { onResult: (result: boolean) => void }) {
  const confirm = useConfirm();
  return (
    <button
      type="button"
      onClick={async () =>
        onResult(
          await confirm({
            title: "Remove bob?",
            body: "Bob will lose access.",
            note: "This cannot be undone.",
            confirmText: "Remove",
          }),
        )
      }
    >
      ask
    </button>
  );
}

function setup() {
  const onResult = vi.fn();
  renderShell(
    <ConfirmProvider>
      <Harness onResult={onResult} />
    </ConfirmProvider>,
  );
  return onResult;
}

test("nothing is shown until a confirmation is requested", () => {
  setup();
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

test("the dialog states the action and resolves true when confirmed", async () => {
  const onResult = setup();
  await userEvent.click(screen.getByRole("button", { name: "ask" }));
  const dialog = screen.getByRole("dialog", { name: "Remove bob?" });
  expect(within(dialog).getByText("Bob will lose access.")).toBeInTheDocument();
  expect(within(dialog).getByText("This cannot be undone.")).toBeInTheDocument();
  await userEvent.click(within(dialog).getByRole("button", { name: "Remove" }));
  expect(onResult).toHaveBeenCalledWith(true);
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

test("cancel, Escape and the close button resolve false", async () => {
  const onResult = setup();
  await userEvent.click(screen.getByRole("button", { name: "ask" }));
  await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(onResult).toHaveBeenLastCalledWith(false);
  await userEvent.click(screen.getByRole("button", { name: "ask" }));
  await userEvent.keyboard("{Escape}");
  expect(onResult).toHaveBeenLastCalledWith(false);
  await userEvent.click(screen.getByRole("button", { name: "ask" }));
  await userEvent.click(screen.getByRole("button", { name: "Close" }));
  expect(onResult).toHaveBeenLastCalledWith(false);
  expect(onResult).toHaveBeenCalledTimes(3);
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

test("the dialog uses the Spanish texts of the app", async () => {
  const onResult = vi.fn();
  renderShell(
    <ConfirmProvider>
      <Harness onResult={onResult} />
    </ConfirmProvider>,
    "/",
    "es",
  );
  await userEvent.click(screen.getByRole("button", { name: "ask" }));
  expect(screen.getByRole("button", { name: "Cancelar" })).toBeInTheDocument();
});
