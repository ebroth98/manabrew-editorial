import { useEffect } from "react";

interface ModalKeyboardHandlers {
  onEnter?: () => void;
  onEscape?: () => void;
}

/**
 * Attaches Enter / Escape keyboard listeners for modal dialogs.
 * Automatically cleans up on unmount or dependency change.
 */
export function useModalKeyboard(
  handlers: ModalKeyboardHandlers,
  deps: unknown[] = [],
) {
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Enter" && handlers.onEnter) {
        e.preventDefault();
        handlers.onEnter();
      } else if (e.key === "Escape" && handlers.onEscape) {
        e.preventDefault();
        handlers.onEscape();
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
}
