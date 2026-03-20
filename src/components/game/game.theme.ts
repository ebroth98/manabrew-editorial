import { useMemo } from "react";
import { usePreferencesStore } from "@/stores/usePreferencesStore";

export interface GameThemeColors {
  activeAction: {
    priority: string;
    turnText: string;
    myTurnRing: string;
    opponentTurnRing: string;
  };
  highlight: string;
  hand: {
    playableBorder: string;
  };
  promptAction: {
    default: string;
    passPriority: string;
    passUntilEnd: string;
    cancel: string;
    pacificAction: string;
  };
}

export const GAME_THEME_COLORS: GameThemeColors = {
  activeAction: {
    priority: "#a855f7",
    turnText: "#f59e0b",
    myTurnRing: "#f59e0b",
    opponentTurnRing: "#f59e0b",
  },
  highlight: "#fb923c",
  hand: {
    playableBorder: "rgba(255, 255, 255, 0.7)",
  },
  promptAction: {
    default: "#7c3aed",
    passPriority:"#7c3aed",
    passUntilEnd: "#5b21b6",
    cancel: "#6b7280",
    pacificAction: "#60a5fa",
  },
};

function cloneThemeColors(colors: GameThemeColors): GameThemeColors {
  return {
    ...colors,
    activeAction: { ...colors.activeAction },
    hand: { ...colors.hand },
    promptAction: { ...colors.promptAction },
  };
}

function hasColorPath(path: string): boolean {
  const segments = path.split(".");
  let cursor: unknown = GAME_THEME_COLORS;
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

function flattenColorLeaves(node: Record<string, unknown>, prefix = ""): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(node)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "string") {
      out[path] = value;
    } else if (typeof value === "object" && value !== null) {
      Object.assign(out, flattenColorLeaves(value as Record<string, unknown>, path));
    }
  }
  return out;
}

export function getDefaultGameThemeColorMap(): Record<string, string> {
  return flattenColorLeaves(GAME_THEME_COLORS as unknown as Record<string, unknown>);
}

export function resolveGameThemeColors(
  overrides: Record<string, string> = {},
): GameThemeColors {
  const merged = cloneThemeColors(GAME_THEME_COLORS);
  for (const [path, value] of Object.entries(overrides)) {
    if (!hasColorPath(path) || typeof value !== "string" || !value.trim()) continue;
    setByPath(merged as unknown as Record<string, unknown>, path, value.trim());
  }
  return merged;
}

export function getGameThemeColors(): GameThemeColors {
  return resolveGameThemeColors(usePreferencesStore.getState().gameThemeColorOverrides);
}

export function useGameThemeColors(): GameThemeColors {
  const overrides = usePreferencesStore((s) => s.gameThemeColorOverrides);
  return useMemo(() => resolveGameThemeColors(overrides), [overrides]);
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
