import { useMemo } from "react";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { THEME_PRESETS } from "@/themes";

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
  cardRing: string;
}

/** 
 * A template for the game theme color structure.
 * Used for path validation and as a fallback schema.
 */
const COLOR_SCHEMA: GameThemeColors = {
  activeAction: { priority: "", active: "" },
  promptAction: { passAction: "", attackAction: "", defenseAction: "", cancel: "" },
  arrow: { attack: "", block: "", hostileTarget: "", friendlyTarget: "" },
  cardRing: "",
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
    cardRing: "#fb923c",
  };

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

export function getDefaultGameThemeColorMap(): Record<string, string> {
  const presetId = usePreferencesStore.getState().appThemePreset;
  const preset = THEME_PRESETS.find((p) => p.id === presetId) || THEME_PRESETS[0]!;
  return { ...preset.gameColors };
}

export function getGameThemeColors(): GameThemeColors {
  return resolveGameThemeColors(usePreferencesStore.getState().gameThemeColorOverrides);
}

export function useGameThemeColors(): GameThemeColors {
  const overrides = usePreferencesStore((s) => s.gameThemeColorOverrides);
  const presetId = usePreferencesStore((s) => s.appThemePreset);
  return useMemo(() => resolveGameThemeColors(overrides, presetId), [overrides, presetId]);
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
