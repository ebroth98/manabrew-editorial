import { useEffect } from "react";
import { useTheme } from "next-themes";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { THEME_PRESETS } from "@/themes";
import type { ThemeColors } from "@/themes";
import {
  flattenGameThemeToCssVars,
  resolveGameThemeColors,
} from "@/components/game/game.theme";

/**
 * Applies the selected theme preset's CSS variables to :root.
 *
 * Two sets of variables are written:
 *   - Radix-style HSL tokens (`--background`, `--foreground`, …) driving
 *     the app chrome via tailwind classes like `bg-background`.
 *   - Game theme tokens (`--pointer-hostile`, `--counter-p1p1`, …) driving
 *     gameplay surfaces via tailwind classes like `bg-pointer-hostile`
 *     (registered in `index.css` `@theme`).
 *
 * Must be mounted once at the app root level.
 */
export function useAppTheme() {
  const presetId = usePreferencesStore((s) => s.appThemePreset);
  const overrides = usePreferencesStore((s) => s.appThemeColorOverrides);
  const gameOverrides = usePreferencesStore((s) => s.gameThemeColorOverrides);
  const { resolvedTheme } = useTheme();

  useEffect(() => {
    const preset = THEME_PRESETS.find((p) => p.id === presetId);
    if (!preset) return;

    const mode = resolvedTheme === "dark" ? "dark" : "light";
    const colors: ThemeColors = preset[mode];

    const root = document.documentElement;
    const allKeys = new Set<string>();

    // 1. Radix HSL tokens (app chrome)
    for (const [key, value] of Object.entries(colors)) {
      root.style.setProperty(`--${key}`, value);
      allKeys.add(key);
    }
    for (const [key, value] of Object.entries(overrides)) {
      if (value) {
        root.style.setProperty(`--${key}`, value);
        allKeys.add(key);
      }
    }

    // 2. Game theme tokens (gameplay surfaces)
    const gameTheme = resolveGameThemeColors(gameOverrides, presetId);
    const gameVars = flattenGameThemeToCssVars(gameTheme);
    for (const [cssKey, value] of Object.entries(gameVars)) {
      root.style.setProperty(cssKey, value);
      // Stored without the leading `--` so cleanup matches the Radix path.
      allKeys.add(cssKey.slice(2));
    }

    return () => {
      for (const key of allKeys) {
        root.style.removeProperty(`--${key}`);
      }
    };
  }, [presetId, resolvedTheme, overrides, gameOverrides]);
}
