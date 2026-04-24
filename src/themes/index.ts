/**
 * Theme barrel — single import point for the full theme system.
 *
 *   import { ThemeColors, GameThemeColors, THEME_PRESETS, ... } from "@/themes";
 *
 * Implementation is split across three files:
 *   - `appTheme.ts`  — app-level (Radix / shadcn) colour interface
 *   - `gameTheme.ts` — game-surface colour interface, resolution logic, utilities
 *   - `presets.ts`    — ThemePreset type, font sizes, and the preset registry
 */

// ── App theme ───────────────────────────────────────────────────────────
export type { ThemeColors } from "./appTheme";

// ── Game theme ──────────────────────────────────────────────────────────
export type { GameThemeColors, ManaLetter } from "./gameTheme";
export {
  resolveGameThemeColors,
  flattenGameThemeToCssVars,
  resolveGameFontSizes,
  getGameThemeColorPaths,
  toPickerHexColor,
  hexToRgb,
  withAlpha,
} from "./gameTheme";

// ── Presets & font sizes ────────────────────────────────────────────────
export type { GameFontSizes, GameThemePresetColors, ThemePreset } from "./presets";
export { DEFAULT_GAME_FONT_SIZES, THEME_PRESETS } from "./presets";
