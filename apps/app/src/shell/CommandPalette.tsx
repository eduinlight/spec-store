import { cn } from "@workspace/ui/lib/utils";
import { Kbd } from "@workspace/ui/patterns";
import { Icon, LockDocument, useModalFocus } from "@workspace/ui/taptap";
import { type KeyboardEvent, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { type Command, type CommandContext, filterGroups, useCommands } from "./commands";

export interface CommandPaletteProps extends CommandContext {
  open: boolean;
  onClose: () => void;
}

const GROUP_LABELS = {
  suggested: "palette.suggested",
  specs: "palette.specs",
  navigation: "palette.navigation",
} as const;

function PaletteDialog({ onClose, ...context }: Omit<CommandPaletteProps, "open">) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const panelRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const baseId = useId();
  useModalFocus(panelRef, true, onClose);
  const groups = filterGroups(useCommands(context, onClose), query);
  const flat: Command[] = groups.flatMap((group) => group.commands);
  const activeIndex = Math.min(active, flat.length - 1);
  const optionId = (index: number) => `${baseId}-option-${index}`;

  const move = (index: number) => {
    const next = Math.max(0, Math.min(index, flat.length - 1));
    setActive(next);
    listRef.current?.querySelector(`#${CSS.escape(optionId(next))}`)?.scrollIntoView?.({
      block: "nearest",
    });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      move(activeIndex + 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      move(activeIndex - 1);
    } else if (event.key === "Home") {
      event.preventDefault();
      move(0);
    } else if (event.key === "End") {
      event.preventDefault();
      move(flat.length - 1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      flat[activeIndex]?.run();
    }
  };

  let offset = 0;
  return createPortal(
    <div className="fixed inset-0 z-[120]">
      <button
        type="button"
        tabIndex={-1}
        aria-hidden="true"
        data-testid="palette-backdrop"
        onClick={onClose}
        className="absolute inset-0 cursor-default border-0 bg-[var(--overlay)] p-0"
      />
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={t("palette.label")}
        tabIndex={-1}
        className="absolute left-1/2 top-[14vh] w-[min(640px,92vw)] -translate-x-1/2 overflow-hidden rounded-lg bg-white text-[var(--neutral-700)] shadow-[var(--elevation-4)] outline-none"
      >
        <div className="flex h-[52px] items-center gap-2.5 border-b border-[var(--neutral-200)] px-4 text-[var(--neutral-500)]">
          <Icon name="Search" size={18} />
          <input
            role="combobox"
            aria-expanded="true"
            aria-controls={`${baseId}-list`}
            aria-activedescendant={flat.length > 0 ? optionId(activeIndex) : undefined}
            aria-label={t("palette.placeholder")}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setActive(0);
            }}
            onKeyDown={onKeyDown}
            placeholder={t("palette.placeholder")}
            className="min-w-0 flex-1 border-0 bg-transparent text-base leading-6 text-[var(--neutral-700)] outline-none"
          />
          <Kbd>esc</Kbd>
        </div>
        <div
          id={`${baseId}-list`}
          ref={listRef}
          role="listbox"
          aria-label={t("palette.results")}
          className="max-h-[min(420px,56vh)] overflow-y-auto px-2 pb-2 pt-1"
        >
          {groups.map((group) => {
            const start = offset;
            offset += group.commands.length;
            const headingId = `${baseId}-${group.id}`;
            return (
              // biome-ignore lint/a11y/useSemanticElements: an option group inside a listbox must be role=group, a fieldset is a form control container
              <div key={group.id} role="group" aria-labelledby={headingId}>
                <div
                  id={headingId}
                  className="px-2 pb-1 pt-2.5 text-xs leading-[18px] text-[var(--neutral-500)]"
                >
                  {t(GROUP_LABELS[group.id])}
                </div>
                {group.commands.map((command, position) => {
                  const index = start + position;
                  const selected = index === activeIndex;
                  return (
                    // biome-ignore lint/a11y/useKeyWithClickEvents: keyboard selection is handled by the search field through aria-activedescendant
                    <div
                      key={command.id}
                      id={optionId(index)}
                      role="option"
                      tabIndex={-1}
                      aria-selected={selected}
                      onClick={command.run}
                      onMouseMove={() => selected || setActive(index)}
                      className={cn(
                        "flex h-10 cursor-pointer items-center gap-3 rounded px-2",
                        selected && "bg-[var(--primary-100)]",
                      )}
                    >
                      <span
                        className={cn(
                          "flex",
                          selected ? "text-[var(--primary-700)]" : "text-[var(--neutral-500)]",
                        )}
                      >
                        {command.icon === "LockDocument" ? (
                          <LockDocument size={16} />
                        ) : (
                          <Icon name={command.icon} size={16} />
                        )}
                      </span>
                      <span
                        className={cn(
                          "whitespace-nowrap text-sm leading-[22px]",
                          command.mono && "font-mono",
                        )}
                      >
                        {command.label}
                      </span>
                      <span className="min-w-0 flex-1 truncate text-[13px] text-[var(--neutral-500)]">
                        {command.sub}
                      </span>
                      {command.keys && (
                        <span className="flex gap-1">
                          {command.keys.map((key) => (
                            <Kbd key={key}>{key}</Kbd>
                          ))}
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            );
          })}
          {flat.length === 0 && (
            <div className="px-4 py-10 text-center text-sm text-[var(--neutral-500)]">
              {t("palette.empty", { query: query.trim() })}
            </div>
          )}
        </div>
        <div className="flex h-10 items-center gap-4 border-t border-[var(--neutral-200)] bg-[var(--neutral-50)] px-4 text-xs text-[var(--neutral-500)]">
          <span className="flex items-center gap-1.5">
            <Kbd>↑</Kbd>
            <Kbd>↓</Kbd>
            {t("palette.hintNavigate")}
          </span>
          <span className="flex items-center gap-1.5">
            <Kbd>↵</Kbd>
            {t("palette.hintOpen")}
          </span>
          <span className="flex items-center gap-1.5">
            <Kbd>esc</Kbd>
            {t("palette.hintClose")}
          </span>
        </div>
      </div>
    </div>,
    document.body,
  );
}

export function CommandPalette({ open, ...rest }: CommandPaletteProps) {
  return open ? <PaletteDialog {...rest} /> : null;
}
