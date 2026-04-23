import { useMemo } from "react";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { THEME_PRESETS, DEFAULT_GAME_FONT_SIZES, type GameFontSizes } from "@/themes";

export type { GameFontSizes };

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
  /** Pointer glow palette. Only two colours: `hostile` for intents that
   *  act against the target (damage, destroy, sacrifice, exile, counter,
   *  tap, discard, …) and `friendly` for supportive intents (buff, heal,
   *  draw, reveal, untap, attach, …). The monochrome icon glyph carries
   *  the specific semantic; the colour only signals valence. See
   *  `intentIsHostile()` in `@/types/promptType`. */
  pointer: {
    hostile: string;
    friendly: string;
  };
  /** Mana-symbol tint used for pip highlights and the dual-land split-tap
   *  button backgrounds. Stored as opaque hex; consumers apply alpha
   *  themselves via `withAlpha`. */
  mana: Record<ManaLetter, string>;
  /** Status-ring / badge colour for each special card state surfaced on
   *  the battlefield (exerted, face-down morph, bestowed aura, copy-token,
   *  transformed DFC, plotted / warp / madness exiled). */
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
  /** Foreground text colour used on top of tinted chips / badges (PT,
   *  counter chips, warning pills). Theme-resolved so future light-mode
   *  variants can flip it to a dark ink. */
  textOnTinted: string;
  /** Subdued label colour used for "empty zone" placeholders over the
   *  canvas surface (graveyard / exile / library zone labels). */
  textMuted: string;
  /** Ghost-placeholder colour used for card-name labels while the card
   *  art is still loading. Slightly brighter than `textMuted`. */
  textGhost: string;
  /** Canvas-level neutrals. `background` paints the empty pixi surface;
   *  `shadow` is the drop-shadow ink; `neutral` is the high-contrast
   *  stroke colour used for arrow strokes / icon outlines. */
  canvas: {
    background: string;
    shadow: string;
    neutral: string;
  };
  /** Placeholder colours used for a card sprite before its texture loads. */
  cardPlaceholder: {
    fill: string;
    stroke: string;
  };
  /** P/T badge backgrounds, keyed by stat-state. Use these only for
   *  actual Power/Toughness badges — do not reuse them as generic
   *  good/bad signals (that's what `success` / `destructive` / `warning`
   *  are for). */
  pt: {
    neutral: string;
    lethal: string;
    buffed: string;
    debuffed: string;
  };
  /** Positive-state indicator — connected, saved, win, good FPS. Green. */
  success: string;
  /** Poison counter / skull icon colour. Traditionally MTG infect green. */
  poison: string;
  /** Life total / heart icon colour. Red. */
  life: string;
  /** Per-counter-type badge colour used by the permanent's counter chips.
   *  `default` is the fallback for unknown counter types. */
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
  /** Player seat colours used by the phase strip indicators and turn tint. */
  playerColors: {
    self: string;
    opponent1: string;
    opponent2: string;
    opponent3: string;
  };
  /** Per-badge icon colours rendered next to the mana pool on the
   *  player panel. Backgroundless — the colour tints both the icon
   *  and its numeric count. */
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

/** Default preset's gameColors map, consulted as a semantic fallback when
 *  the active preset doesn't declare a given game-theme key. This
 *  keeps all concrete colours inside theme files (no hardcoded palette
 *  duplicated inside this module). */
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
 * A template for the game theme color structure.
 * Used for path validation and as a fallback schema.
 */
const COLOR_SCHEMA: GameThemeColors = {
  activeAction: { priority: "", active: "" },
  promptAction: { passAction: "", attackAction: "", defenseAction: "", cancel: "" },
  arrow: { attack: "", block: "", hostileTarget: "", friendlyTarget: "" },
  pointer: { hostile: "", friendly: "" },
  mana: { W: "", U: "", B: "", R: "", G: "", C: "" },
  cardStatus: {
    exerted: "", morph: "", bestow: "", token: "",
    transformed: "", plotted: "", madness: "", warped: "",
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
    default: "", p1p1: "", m1m1: "", loyalty: "", charge: "", quest: "",
    study: "", lore: "", age: "", time: "", fade: "", level: "",
    storage: "", mining: "", brick: "", depletion: "", page: "",
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

/**
 * Maps a flat string record (from theme presets or user overrides) into the structured GameThemeColors object.
 * Handles legacy path resolution and provides absolute fallbacks for missing keys.
 */
export function resolveGameThemeColors(
  overrides: Record<string, string> = {},
  presetId?: string,
): GameThemeColors {
  // 1. Determine which preset to use as the base
  const activePresetId = presetId ?? usePreferencesStore.getState().appThemePreset;
  const preset = THEME_PRESETS.find((p) => p.id === activePresetId) || THEME_PRESETS[0]!;
  const presetColors = (preset.gameColors || {}) as unknown as Record<string, string>;

  // 2. Start with an absolute fallback base (never hardcoded in exported constants)
  const merged: GameThemeColors = {
    activeAction: {
      priority: "#a855f7",
      active: "#fb923c",
    },
    promptAction: {
      passAction: "#7c3aed",
      attackAction: "#dc2626",
      defenseAction: "#2563eb",
      cancel: "#6b7280",
    },
    arrow: {
      attack: "rgba(255, 138, 0, 0.88)",
      block: "rgba(90, 150, 255, 0.88)",
      hostileTarget: "rgba(210, 40, 40, 0.88)",
      friendlyTarget: "rgba(90, 150, 255, 0.88)",
    },
    pointer: { hostile: "", friendly: "" },
    mana: { W: "", U: "", B: "", R: "", G: "", C: "" },
    cardStatus: {
      exerted: "", morph: "", bestow: "", token: "",
      transformed: "", plotted: "", madness: "", warped: "",
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
      default: "", p1p1: "", m1m1: "", loyalty: "", charge: "", quest: "",
      study: "", lore: "", age: "", time: "", fade: "", level: "",
      storage: "", mining: "", brick: "", depletion: "", page: "",
    },
    cardRing: "#fb923c",
    playerColors: {
      self: "#4ade80",
      opponent1: "#facc15",
      opponent2: "#60a5fa",
      opponent3: "#c084fc",
    },
    badges: {
      monarch: "#facc15",
      initiative: "#60a5fa",
      poison: "#65a30d",
      energy: "#fbbf24",
      commanderDamage: "#dc2626",
      hand: "#9ca3af",
      radiation: "#84cc16",
      cityBlessing: "#f59e0b",
      ring: "#a78bfa",
      speed: "#f97316",
    },
  };

  // 2a. Semantic fallback: seed pointer colours (and any other
  // defaultable key) from the default theme preset before we apply the
  // active preset's overrides. This keeps concrete colours inside theme
  // files and lets other presets opt out of restating every intent.
  if (activePresetId !== "default") {
    for (const [path, value] of Object.entries(DEFAULT_PRESET_GAME_COLORS)) {
      if (!hasColorPath(path) || !value.trim()) continue;
      setByPath(merged as unknown as Record<string, unknown>, path, value.trim());
    }
  }

  // 3. Apply preset colors
  for (const [path, value] of Object.entries(presetColors)) {
    if (!hasColorPath(path) || typeof value !== "string" || !value.trim()) continue;
    setByPath(merged as unknown as Record<string, unknown>, path, value.trim());
  }

  // 4. Apply user overrides
  for (const [path, value] of Object.entries(overrides)) {
    if (!hasColorPath(path) || typeof value !== "string" || !value.trim()) continue;
    setByPath(merged as unknown as Record<string, unknown>, path, value.trim());
  }

  // 5. Apply legacy fallback logic for derived fields if they are still using default values
  const combined: Record<string, string> = { ...presetColors, ...overrides };

  // activeAction.active resolution
  const explicitActive = overrides["activeAction.active"]?.trim() || presetColors["activeAction.active"]?.trim();
  if (!explicitActive) {
    const legacyActivePaths = [
      "activeAction.turnText",
      "activeAction.myTurnRing",
      "activeAction.opponentTurnRing",
      "highlight",
      "hand.playableBorder",
    ] as const;
    for (const path of legacyActivePaths) {
      const value = combined[path]?.trim();
      if (!value) continue;
      if (path === "hand.playableBorder" && !value.startsWith("#")) continue;
      merged.activeAction.active = value;
      break;
    }
  }

  // promptAction.passAction resolution
  const explicitPassAction = overrides["promptAction.passAction"]?.trim() || presetColors["promptAction.passAction"]?.trim();
  if (!explicitPassAction) {
    const legacyPassPaths = [
      "promptAction.default",
      "promptAction.passPriority",
      "promptAction.passUntilEnd",
      "promptAction.pacificAction",
    ] as const;
    for (const path of legacyPassPaths) {
      const value = combined[path]?.trim();
      if (!value) continue;
      merged.promptAction.passAction = value;
      break;
    }
  }

  // promptAction.attackAction resolution
  const explicitAttackAction = overrides["promptAction.attackAction"]?.trim() || presetColors["promptAction.attackAction"]?.trim();
  if (!explicitAttackAction) {
    const legacyAttackPaths = ["promptAction.attack", "promptAction.secondary"] as const;
    for (const path of legacyAttackPaths) {
      const value = combined[path]?.trim();
      if (!value) continue;
      merged.promptAction.attackAction = value;
      break;
    }
  }

  // promptAction.defenseAction resolution
  const explicitDefenseAction = overrides["promptAction.defenseAction"]?.trim() || presetColors["promptAction.defenseAction"]?.trim();
  if (!explicitDefenseAction) {
    const legacyDefensePaths = ["promptAction.defense", "promptAction.primary", "promptAction.pacificAction"] as const;
    for (const path of legacyDefensePaths) {
      const value = combined[path]?.trim();
      if (!value) continue;
      merged.promptAction.defenseAction = value;
      break;
    }
  }

  // cardRing resolution
  const explicitCardRing = overrides["cardRing"]?.trim() || presetColors["cardRing"]?.trim();
  if (explicitCardRing) {
    merged.cardRing = explicitCardRing;
  } else {
    merged.cardRing = merged.activeAction.active;
  }

  return merged;
}

/**
 * Base game theme colors exported for legacy compatibility.
 * Now derived dynamically from the default theme.
 */
export const GAME_THEME_COLORS: GameThemeColors = resolveGameThemeColors({}, "default");

/**
 * Flatten the nested `GameThemeColors` object into CSS-variable-ready
 * key/value pairs. Object paths become dash-separated, camelCase is
 * converted to kebab-case, and each key is prefixed with `--` so the
 * result can be handed directly to `element.style.setProperty`.
 *
 * Example: `{ pointer: { hostile: "red" } }` → `{ "--pointer-hostile": "red" }`.
 */
export function flattenGameThemeToCssVars(
  theme: GameThemeColors,
): Record<string, string> {
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

export function getDefaultGameThemeColorMap(): Record<string, string> {
  const presetId = usePreferencesStore.getState().appThemePreset;
  const preset = THEME_PRESETS.find((p) => p.id === presetId) || THEME_PRESETS[0]!;
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(preset.gameColors)) {
    if (typeof value === "string") out[key] = value;
  }
  return out;
}

export function getGameThemeColors(): GameThemeColors {
  return resolveGameThemeColors(usePreferencesStore.getState().gameThemeColorOverrides);
}

export function useGameThemeColors(): GameThemeColors {
  const overrides = usePreferencesStore((s) => s.gameThemeColorOverrides);
  const presetId = usePreferencesStore((s) => s.appThemePreset);
  return useMemo(() => resolveGameThemeColors(overrides, presetId), [overrides, presetId]);
}

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

export function getGameFontSizes(): GameFontSizes {
  return resolveGameFontSizes();
}

export function useGameFontSizes(): GameFontSizes {
  const presetId = usePreferencesStore((s) => s.appThemePreset);
  return useMemo(() => resolveGameFontSizes(presetId), [presetId]);
}

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
