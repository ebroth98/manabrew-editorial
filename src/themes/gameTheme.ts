/**
 * Game-surface theme colours and resolution logic.
 *
 * `GameThemeColors` is the single source of truth for every colour token
 * consumed by the game canvas, Pixi renderers, card sprites, prompt
 * buttons, and in-game panels.  Theme presets supply flat dot-notation
 * keys (e.g. `"pointer.hostile"`) that are validated against the schema
 * below; `resolveGameThemeColors` merges preset defaults → active preset
 * → user overrides into a fully-resolved `GameThemeColors` object.
 */

import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { THEME_PRESETS, DEFAULT_GAME_FONT_SIZES, type GameFontSizes } from "./presets";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ManaLetter = "W" | "U" | "B" | "R" | "G" | "C";

export interface GameThemeColors {
  activeAction: {
    priority: string;
    active: string;
  };
  promptAction: {
    passAction: string;
    attackAction: string;
    defenseAction: string;
    cancel: string;
  };
  arrow: {
    attack: string;
    block: string;
    hostileTarget: string;
    friendlyTarget: string;
  };
  pointer: {
    hostile: string;
    friendly: string;
  };
  mana: Record<ManaLetter, string>;
  cardStatus: {
    exerted: string;
    morph: string;
    bestow: string;
    token: string;
    transformed: string;
    plotted: string;
    madness: string;
    warped: string;
  };
  textOnTinted: string;
  textMuted: string;
  textGhost: string;
  canvas: {
    background: string;
    shadow: string;
    neutral: string;
  };
  cardPlaceholder: {
    fill: string;
    stroke: string;
  };
  pt: {
    neutral: string;
    lethal: string;
    buffed: string;
    debuffed: string;
  };
  success: string;
  poison: string;
  life: string;
  counter: {
    default: string;
    p1p1: string;
    m1m1: string;
    loyalty: string;
    charge: string;
    quest: string;
    study: string;
    lore: string;
    age: string;
    time: string;
    fade: string;
    level: string;
    storage: string;
    mining: string;
    brick: string;
    depletion: string;
    page: string;
  };
  cardRing: string;
  playerColors: {
    self: string;
    opponent1: string;
    opponent2: string;
    opponent3: string;
  };
  badges: {
    monarch: string;
    initiative: string;
    poison: string;
    energy: string;
    commanderDamage: string;
    hand: string;
    radiation: string;
    cityBlessing: string;
    ring: string;
    speed: string;
  };
}

// ---------------------------------------------------------------------------
// Schema — drives path validation + enumeration
// ---------------------------------------------------------------------------

const COLOR_SCHEMA: GameThemeColors = {
  activeAction: { priority: "", active: "" },
  promptAction: { passAction: "", attackAction: "", defenseAction: "", cancel: "" },
  arrow: { attack: "", block: "", hostileTarget: "", friendlyTarget: "" },
  pointer: { hostile: "", friendly: "" },
  mana: { W: "", U: "", B: "", R: "", G: "", C: "" },
  cardStatus: {
    exerted: "",
    morph: "",
    bestow: "",
    token: "",
    transformed: "",
    plotted: "",
    madness: "",
    warped: "",
  },
  textOnTinted: "",
  textMuted: "",
  textGhost: "",
  canvas: { background: "", shadow: "", neutral: "" },
  cardPlaceholder: { fill: "", stroke: "" },
  pt: { neutral: "", lethal: "", buffed: "", debuffed: "" },
  success: "",
  poison: "",
  life: "",
  counter: {
    default: "",
    p1p1: "",
    m1m1: "",
    loyalty: "",
    charge: "",
    quest: "",
    study: "",
    lore: "",
    age: "",
    time: "",
    fade: "",
    level: "",
    storage: "",
    mining: "",
    brick: "",
    depletion: "",
    page: "",
  },
  cardRing: "",
  playerColors: { self: "", opponent1: "", opponent2: "", opponent3: "" },
  badges: {
    monarch: "",
    initiative: "",
    poison: "",
    energy: "",
    commanderDamage: "",
    hand: "",
    radiation: "",
    cityBlessing: "",
    ring: "",
    speed: "",
  },
};

/** Return every valid dot-notation leaf path from `COLOR_SCHEMA`.
 *  Used by Settings and `getDefaultGameThemeColorMap` to enumerate
 *  the canonical key set without duplicating the schema. */
export function getGameThemeColorPaths(): string[] {
  const paths: string[] = [];
  const walk = (obj: unknown, prefix: string): void => {
    if (typeof obj === "string") {
      paths.push(prefix);
      return;
    }
    if (obj != null && typeof obj === "object") {
      for (const [key, value] of Object.entries(obj as Record<string, unknown>)) {
        walk(value, prefix ? `${prefix}.${key}` : key);
      }
    }
  };
  walk(COLOR_SCHEMA, "");
  return paths;
}

function hasColorPath(path: string): boolean {
  const segments = path.split(".");
  let cursor: unknown = COLOR_SCHEMA;
  for (const segment of segments) {
    if (typeof cursor !== "object" || cursor === null || !(segment in cursor)) {
      return false;
    }
    cursor = (cursor as Record<string, unknown>)[segment];
  }
  return typeof cursor === "string";
}

function setByPath(target: Record<string, unknown>, path: string, value: string): void {
  const segments = path.split(".");
  const lastIndex = segments.length - 1;
  let cursor: Record<string, unknown> = target;
  for (let i = 0; i < lastIndex; i += 1) {
    cursor = cursor[segments[i]!] as Record<string, unknown>;
  }
  cursor[segments[lastIndex]!] = value;
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/** Default preset's gameColors map, consulted as a semantic fallback when
 *  the active preset doesn't declare a given game-theme key. */
const DEFAULT_PRESET_GAME_COLORS: Record<string, string> = (() => {
  const defaultPreset = THEME_PRESETS.find((p) => p.id === "default");
  const out: Record<string, string> = {};
  if (!defaultPreset) return out;
  for (const [key, value] of Object.entries(defaultPreset.gameColors)) {
    if (typeof value === "string") out[key] = value;
  }
  return out;
})();

/**
 * Maps a flat string record (from theme presets or user overrides) into
 * the structured `GameThemeColors` object.
 */
export function resolveGameThemeColors(
  overrides: Record<string, string> = {},
  presetId?: string,
): GameThemeColors {
  const activePresetId = presetId ?? usePreferencesStore.getState().appThemePreset;
  const preset = THEME_PRESETS.find((p) => p.id === activePresetId) || THEME_PRESETS[0]!;
  const presetColors = (preset.gameColors || {}) as unknown as Record<string, string>;

  // Start with an empty shell — the default preset fills every key.
  const merged: GameThemeColors = structuredClone(COLOR_SCHEMA) as GameThemeColors;

  // Seed from the default preset (provides all fallback values)
  for (const [path, value] of Object.entries(DEFAULT_PRESET_GAME_COLORS)) {
    if (!hasColorPath(path) || !value.trim()) continue;
    setByPath(merged as unknown as Record<string, unknown>, path, value.trim());
  }

  // Apply preset colors
  for (const [path, value] of Object.entries(presetColors)) {
    if (!hasColorPath(path) || typeof value !== "string" || !value.trim()) continue;
    setByPath(merged as unknown as Record<string, unknown>, path, value.trim());
  }

  // Apply user overrides
  for (const [path, value] of Object.entries(overrides)) {
    if (!hasColorPath(path) || typeof value !== "string" || !value.trim()) continue;
    setByPath(merged as unknown as Record<string, unknown>, path, value.trim());
  }

  return merged;
}

// ---------------------------------------------------------------------------
// CSS variable emission
// ---------------------------------------------------------------------------

/**
 * Flatten the nested `GameThemeColors` object into CSS-variable-ready
 * key/value pairs.  Object paths become dash-separated, camelCase is
 * converted to kebab-case, and each key is prefixed with `--`.
 *
 * Example: `{ pointer: { hostile: "red" } }` -> `{ "--pointer-hostile": "red" }`.
 */
export function flattenGameThemeToCssVars(theme: GameThemeColors): Record<string, string> {
  const out: Record<string, string> = {};
  const walk = (value: unknown, prefix: string): void => {
    if (typeof value === "string") {
      if (value.trim()) out[`--${prefix}`] = value;
      return;
    }
    if (value == null || typeof value !== "object") return;
    for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
      const segment = camelToKebab(key).toLowerCase();
      const nextPrefix = prefix ? `${prefix}-${segment}` : segment;
      walk(nested, nextPrefix);
    }
  };
  walk(theme, "");
  return out;
}

function camelToKebab(s: string): string {
  return s.replace(/[A-Z]/g, (char, index) =>
    index === 0 ? char.toLowerCase() : `-${char.toLowerCase()}`,
  );
}

// ---------------------------------------------------------------------------
// Font sizes
// ---------------------------------------------------------------------------

/** Resolve the active preset's `gameFontSizes`, inheriting unset keys
 *  from the default preset, then from `DEFAULT_GAME_FONT_SIZES`. */
export function resolveGameFontSizes(presetId?: string): GameFontSizes {
  const activePresetId = presetId ?? usePreferencesStore.getState().appThemePreset;
  const active = THEME_PRESETS.find((p) => p.id === activePresetId);
  const fallback = THEME_PRESETS.find((p) => p.id === "default");
  return {
    ...DEFAULT_GAME_FONT_SIZES,
    ...(fallback?.gameFontSizes ?? {}),
    ...(active?.gameFontSizes ?? {}),
  };
}

// ---------------------------------------------------------------------------
// Colour utilities
// ---------------------------------------------------------------------------

function normalizeHexColor(hex: string): string {
  const value = hex.trim().replace("#", "");
  if (/^[\da-fA-F]{3}$/.test(value)) {
    return `#${value
      .split("")
      .map((char) => `${char}${char}`)
      .join("")
      .toLowerCase()}`;
  }
  if (/^[\da-fA-F]{6}$/.test(value)) {
    return `#${value.toLowerCase()}`;
  }
  return "#000000";
}

export function toPickerHexColor(value: string): string {
  const trimmed = value.trim();
  if (trimmed.startsWith("#")) return normalizeHexColor(trimmed);
  const rgbaMatch = trimmed.match(
    /^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})(?:\s*,\s*(0|1|0?\.\d+))?\s*\)$/i,
  );
  if (rgbaMatch) {
    const r = Math.min(255, Number.parseInt(rgbaMatch[1]!, 10));
    const g = Math.min(255, Number.parseInt(rgbaMatch[2]!, 10));
    const b = Math.min(255, Number.parseInt(rgbaMatch[3]!, 10));
    return `#${[r, g, b].map((n) => n.toString(16).padStart(2, "0")).join("")}`;
  }
  return "#000000";
}

export function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const normalized = normalizeHexColor(hex);
  const raw = normalized.slice(1);
  return {
    r: Number.parseInt(raw.slice(0, 2), 16),
    g: Number.parseInt(raw.slice(2, 4), 16),
    b: Number.parseInt(raw.slice(4, 6), 16),
  };
}

export function withAlpha(hex: string, alpha: number): string {
  const { r, g, b } = hexToRgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
