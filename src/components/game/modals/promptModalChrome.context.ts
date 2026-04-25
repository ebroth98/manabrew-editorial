import { createContext } from "react";

interface PromptModalChromeContextValue {
  showMinimize: boolean;
  onMinimize?: () => void;
}

export const PromptModalChromeContext = createContext<PromptModalChromeContextValue>({
  showMinimize: false,
});
