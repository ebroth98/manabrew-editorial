import type { CSSProperties } from "react";
import {
  GAME_THEME_COLORS,
  useGameThemeColors,
  withAlpha,
} from "@/components/game/game.theme";

export function usePromptActionColors() {
  return useGameThemeColors().promptAction;
}

export function getPromptActionButtonStyle(baseColor: string): CSSProperties {
  const resolved = baseColor || GAME_THEME_COLORS.promptAction.passAction;
  const shadow = `0 4px 14px ${withAlpha(resolved, 0.28)}`;

  return {
    border: "0",
    color: GAME_THEME_COLORS.textOnTinted,
    backgroundColor: resolved,
    boxShadow: shadow,
  };
}
