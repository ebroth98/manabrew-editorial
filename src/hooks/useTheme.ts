import { useEffect, useMemo } from "react";
import { useTheme as useNextTheme } from "next-themes";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { THEME_PRESETS } from "@/themes";
import type { ThemeColors } from "@/themes";
import {
  resolveGameThemeColors,
  flattenGameThemeToCssVars,
  resolveGameFontSizes,
  type GameThemeColors,
} from "@/components/game/game.theme";
import type { GameFontSizes } from "@/themes";
export type { GameThemeColors } from "@/components/game/game.theme";
export type { GameFontSizes } from "@/themes";

export interface AppTheme {
  app: ThemeColors;
  game: GameThemeColors;
  fontSizes: GameFontSizes;
  /** Flat CSS-variable map for game theme colours (keys include `--` prefix). */
  gameCssVars: Record<string, string>;
}

// ---------------------------------------------------------------------------
// Imperative accessor — cached, kept in sync via a preferences subscription.
// Used by Pixi and other non-React code that cannot call hooks.
// ---------------------------------------------------------------------------

function buildTheme(): AppTheme {
  const { appThemePreset, gameThemeColorOverrides } =
    usePreferencesStore.getState();
  const preset =
    THEME_PRESETS.find((p) => p.id === appThemePreset) ?? THEME_PRESETS[0]!;
  const app = preset.dark;
  const game = resolveGameThemeColors(gameThemeColorOverrides, appThemePreset);
  const fontSizes = resolveGameFontSizes(appThemePreset);
  const gameCssVars = flattenGameThemeToCssVars(game);
  return { app, game, fontSizes, gameCssVars };
}

let cachedTheme: AppTheme = buildTheme();
let prevPreset = usePreferencesStore.getState().appThemePreset;
let prevGameOverrides = usePreferencesStore.getState().gameThemeColorOverrides;

usePreferencesStore.subscribe(() => {
  const { appThemePreset, gameThemeColorOverrides } =
    usePreferencesStore.getState();
  if (
    appThemePreset !== prevPreset ||
    gameThemeColorOverrides !== prevGameOverrides
  ) {
    prevPreset = appThemePreset;
    prevGameOverrides = gameThemeColorOverrides;
    cachedTheme = buildTheme();
  }
});

/** Non-reactive accessor for imperative / Pixi code. */
export function getTheme(): AppTheme {
  return cachedTheme;
}
export function getDefaultGameThemeColorMap(): Record<string, string> {
  const presetId = usePreferencesStore.getState().appThemePreset;
  const preset =
    THEME_PRESETS.find((p) => p.id === presetId) ?? THEME_PRESETS[0]!;
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(preset.gameColors)) {
    if (typeof value === "string") out[key] = value;
  }
  return out;
}
export function useTheme(): AppTheme {
  const presetId = usePreferencesStore((s) => s.appThemePreset);
  const appOverrides = usePreferencesStore((s) => s.appThemeColorOverrides);
  const gameOverrides = usePreferencesStore((s) => s.gameThemeColorOverrides);
  const { resolvedTheme } = useNextTheme();

  const theme = useMemo(() => {
    const preset =
      THEME_PRESETS.find((p) => p.id === presetId) ?? THEME_PRESETS[0]!;
    const mode = resolvedTheme === "dark" ? "dark" : "light";
    const app = preset[mode];
    const game = resolveGameThemeColors(gameOverrides, presetId);
    const fontSizes = resolveGameFontSizes(presetId);
    const gameCssVars = flattenGameThemeToCssVars(game);
    return { app, game, fontSizes, gameCssVars };
  }, [presetId, gameOverrides, resolvedTheme]);

  // Write CSS variables onto :root (idempotent — no cleanup needed since the
  // full set of keys is always written on every change).
  useEffect(() => {
    const preset = THEME_PRESETS.find((p) => p.id === presetId);
    if (!preset) return;

    const mode = resolvedTheme === "dark" ? "dark" : "light";
    const colors: ThemeColors = preset[mode];
    const root = document.documentElement;

    for (const [key, value] of Object.entries(colors)) {
      root.style.setProperty(`--${key}`, value);
    }
    for (const [key, value] of Object.entries(appOverrides)) {
      if (value) root.style.setProperty(`--${key}`, value);
    }
    for (const [cssKey, value] of Object.entries(theme.gameCssVars)) {
      root.style.setProperty(cssKey, value);
    }
  }, [presetId, resolvedTheme, appOverrides, theme.gameCssVars]);

  return theme;
}
