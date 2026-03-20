import { useEffect } from "react";
import { useTheme } from "next-themes";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { THEME_PRESETS } from "@/themes";
import type { ThemeColors } from "@/themes";

/**
 * Applies the selected theme preset's CSS variables to :root.
 * Must be mounted once at the app root level.
 */
export function useAppTheme() {
  const presetId = usePreferencesStore((s) => s.appThemePreset);
  const overrides = usePreferencesStore((s) => s.appThemeColorOverrides);
  const { resolvedTheme } = useTheme();

  useEffect(() => {
    const preset = THEME_PRESETS.find((p) => p.id === presetId);
    if (!preset) return;

    const mode = resolvedTheme === "dark" ? "dark" : "light";
    const colors: ThemeColors = preset[mode];

    const root = document.documentElement;
    const allKeys = new Set<string>();

    // Apply preset colors
    for (const [key, value] of Object.entries(colors)) {
      root.style.setProperty(`--${key}`, value);
      allKeys.add(key);
    }

    // Apply user overrides on top
    for (const [key, value] of Object.entries(overrides)) {
      if (value) {
        root.style.setProperty(`--${key}`, value);
        allKeys.add(key);
      }
    }

    return () => {
      for (const key of allKeys) {
        root.style.removeProperty(`--${key}`);
      }
    };
  }, [presetId, resolvedTheme, overrides]);
}
