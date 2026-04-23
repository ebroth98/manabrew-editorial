import type { CSSProperties } from "react";
import { withAlpha } from "@/components/game/game.theme";
import { useTheme, getTheme } from "@/hooks/useTheme";

export function usePromptActionColors() {
  return useTheme().game.promptAction;
}

export function getPromptActionButtonStyle(baseColor: string): CSSProperties {
  const theme = getTheme().game;
  const resolved = baseColor || theme.promptAction.passAction;
  const shadow = `0 4px 14px ${withAlpha(resolved, 0.28)}`;

  return {
    border: "0",
    color: theme.textOnTinted,
    backgroundColor: resolved,
    boxShadow: shadow,
  };
}
