import { useGameUIStore } from "@/stores/useGameUIStore";
import { Children, createContext, type ReactNode, useEffect, useMemo } from "react";

interface PromptModalChromeContextValue {
  showMinimize: boolean;
  onMinimize?: () => void;
}

export const PromptModalChromeContext = createContext<PromptModalChromeContextValue>({
  showMinimize: false,
});

interface PromptModalControllerProps {
  isActive: boolean;
  promptStateKey: unknown;
  children: ReactNode;
}

export function PromptModalController({
  isActive,
  promptStateKey,
  children,
}: PromptModalControllerProps) {
  const promptModalHidden = useGameUIStore((s) => s.promptModalHidden);
  const showPromptModal = useGameUIStore((s) => s.showPromptModal);
  const hidePromptModal = useGameUIStore((s) => s.hidePromptModal);

  const activeChildren = useMemo(() => Children.toArray(children).filter(Boolean), [children]);

  const isOpen = !promptModalHidden;

  useEffect(() => {
    if (isActive) {
      showPromptModal();
    }
  }, [isActive, promptStateKey, showPromptModal]);

  if (!isActive) {
    return null;
  }

  if (activeChildren.length !== 1) {
    throw new Error(
      `PromptModalController expected exactly 1 active modal child, got ${activeChildren.length}`,
    );
  }

  if (!isOpen) {
    return null;
  }

  return (
    <PromptModalChromeContext.Provider value={{ showMinimize: true, onMinimize: hidePromptModal }}>
      {activeChildren[0]}
    </PromptModalChromeContext.Provider>
  );
}
