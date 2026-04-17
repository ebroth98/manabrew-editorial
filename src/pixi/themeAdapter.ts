import type { GameThemeColors } from "@/components/game/game.theme";

export interface PixiThemeColors {
  activeAction: {
    priority: number;
    active: number;
  };
  promptAction: {
    passAction: number;
    attackAction: number;
    defenseAction: number;
    cancel: number;
  };
  arrow: {
    attack: { color: number; alpha: number };
    block: { color: number; alpha: number };
    hostileTarget: { color: number; alpha: number };
    friendlyTarget: { color: number; alpha: number };
  };
  cardRing: number;
}

function parseColor(value: string): { color: number; alpha: number } {
  const rgbaMatch = value.match(
    /^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})(?:\s*,\s*([\d.]+))?\s*\)$/i,
  );
  if (rgbaMatch) {
    const r = Math.min(255, parseInt(rgbaMatch[1]!, 10));
    const g = Math.min(255, parseInt(rgbaMatch[2]!, 10));
    const b = Math.min(255, parseInt(rgbaMatch[3]!, 10));
    const a = rgbaMatch[4] != null ? parseFloat(rgbaMatch[4]) : 1;
    return { color: (r << 16) | (g << 8) | b, alpha: a };
  }

  let hex = value.trim().replace("#", "");
  if (hex.length === 3) {
    hex = hex.split("").map((c) => c + c).join("");
  }
  if (hex.length === 6) {
    return { color: parseInt(hex, 16), alpha: 1 };
  }

  return { color: 0x000000, alpha: 1 };
}

function parseHex(value: string): number {
  return parseColor(value).color;
}

export function adaptTheme(theme: GameThemeColors): PixiThemeColors {
  return {
    activeAction: {
      priority: parseHex(theme.activeAction.priority),
      active: parseHex(theme.activeAction.active),
    },
    promptAction: {
      passAction: parseHex(theme.promptAction.passAction),
      attackAction: parseHex(theme.promptAction.attackAction),
      defenseAction: parseHex(theme.promptAction.defenseAction),
      cancel: parseHex(theme.promptAction.cancel),
    },
    arrow: {
      attack: parseColor(theme.arrow.attack),
      block: parseColor(theme.arrow.block),
      hostileTarget: parseColor(theme.arrow.hostileTarget),
      friendlyTarget: parseColor(theme.arrow.friendlyTarget),
    },
    cardRing: parseHex(theme.cardRing),
  };
}
