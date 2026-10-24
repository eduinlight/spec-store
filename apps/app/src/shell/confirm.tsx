import { Button, Dialog, Icon } from "@workspace/ui/taptap";
import { createContext, type ReactNode, useCallback, useContext, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

export interface ConfirmOptions {
  title: string;
  body: ReactNode;
  note?: string;
  confirmText: string;
}

export type Confirm = (options: ConfirmOptions) => Promise<boolean>;

interface Pending {
  options: ConfirmOptions;
  resolve: (result: boolean) => void;
}

const ConfirmContext = createContext<Confirm | null>(null);

export function ConfirmProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const [pending, setPending] = useState<Pending | null>(null);
  const current = useRef<Pending | null>(null);

  const settle = useCallback((result: boolean) => {
    current.current?.resolve(result);
    current.current = null;
    setPending(null);
  }, []);

  const confirm = useCallback<Confirm>(
    (options) =>
      new Promise<boolean>((resolve) => {
        current.current?.resolve(false);
        const next = { options, resolve };
        current.current = next;
        setPending(next);
      }),
    [],
  );

  return (
    <ConfirmContext.Provider value={confirm}>
      {children}
      <Dialog
        open={pending !== null}
        title={pending?.options.title}
        size={440}
        onCancel={() => settle(false)}
        footer={
          <>
            <Button variant="outline" onClick={() => settle(false)}>
              {t("app.cancel")}
            </Button>
            <Button danger onClick={() => settle(true)}>
              {pending?.options.confirmText}
            </Button>
          </>
        }
      >
        <div className="flex items-start gap-3">
          <span className="flex size-8 flex-none items-center justify-center rounded-full bg-[var(--danger-100)] text-[var(--danger-600)]">
            <Icon name="Attention" size={18} />
          </span>
          <div className="flex flex-col gap-2 text-sm leading-[22px] text-[var(--neutral-600)]">
            <span>{pending?.options.body}</span>
            {pending?.options.note && (
              <span className="text-xs leading-[18px] text-[var(--neutral-500)]">
                {pending.options.note}
              </span>
            )}
          </div>
        </div>
      </Dialog>
    </ConfirmContext.Provider>
  );
}

export function useConfirm(): Confirm {
  const confirm = useContext(ConfirmContext);
  if (!confirm) {
    throw new Error("useConfirm needs a ConfirmProvider");
  }
  return confirm;
}
