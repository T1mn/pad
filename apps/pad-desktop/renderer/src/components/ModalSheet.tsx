import { useEffect, useRef } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
import { createPortal } from "react-dom";

const focusableSelector = [
  "button:not(:disabled)",
  "input:not(:disabled)",
  "select:not(:disabled)",
  "textarea:not(:disabled)",
  "[href]",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

export function ModalSheet({
  labelledBy,
  describedBy,
  className = "",
  busy = false,
  onDismiss,
  children,
}: {
  labelledBy: string;
  describedBy?: string;
  className?: string;
  busy?: boolean;
  onDismiss(): void;
  children: ReactNode;
}) {
  const dialogRef = useRef<HTMLElement>(null);
  const overlayRoot = document.getElementById("overlay-root") ?? document.body;

  useEffect(() => {
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const siblings = Array.from(overlayRoot.parentElement?.children ?? [])
      .filter((element): element is HTMLElement => element instanceof HTMLElement && element !== overlayRoot)
      .map((element) => ({
        element,
        inert: element.inert,
        ariaHidden: element.getAttribute("aria-hidden"),
      }));
    siblings.forEach(({ element }) => {
      element.inert = true;
      element.setAttribute("aria-hidden", "true");
    });
    dialogRef.current?.querySelector<HTMLElement>(focusableSelector)?.focus();
    return () => {
      siblings.forEach(({ element, inert, ariaHidden }) => {
        element.inert = inert;
        if (ariaHidden === null) element.removeAttribute("aria-hidden");
        else element.setAttribute("aria-hidden", ariaHidden);
      });
      previousFocus?.focus();
    };
  }, [overlayRoot]);

  function handleKeyDown(event: ReactKeyboardEvent<HTMLElement>) {
    if (event.key === "Escape" && !busy) {
      event.preventDefault();
      onDismiss();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = Array.from(
      dialogRef.current?.querySelectorAll<HTMLElement>(focusableSelector) ?? [],
    ).filter((element) => !element.hidden && element.getAttribute("aria-hidden") !== "true");
    if (focusable.length === 0) {
      event.preventDefault();
      dialogRef.current?.focus();
      return;
    }
    const first = focusable[0]!;
    const last = focusable[focusable.length - 1]!;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  return createPortal(
    <div className="auth-sheet-backdrop" role="presentation">
      <section
        ref={dialogRef}
        className={`auth-sheet${className ? ` ${className}` : ""}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        aria-describedby={describedBy}
        tabIndex={-1}
        onKeyDown={handleKeyDown}
      >
        {children}
      </section>
    </div>,
    overlayRoot,
  );
}
