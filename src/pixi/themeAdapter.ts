import type { GameThemeColors, ManaLetter } from "@/components/game/game.theme";

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
  /** Pointer glow palette — only two colours. `intentIsHostile()` picks
   *  which one applies for a given intent. */
  pointer: {
    hostile: { color: number; alpha: number };
    friendly: { color: number; alpha: number };
  };
  /** Mana-letter tints parsed from the theme; opaque number colour. */
  mana: Record<ManaLetter, number>;
  /** Badge colour for each tracked card-status indicator. */
  cardStatus: {
    exerted: number;
    morph: number;
    bestow: number;
    token: number;
    transformed: number;
    plotted: number;
    madness: number;
    warped: number;
  };
  /** Foreground text colour used on tinted chips/badges, parsed as both
   *  the raw string (for `TextStyle.fill`) and the numeric value. */
  textOnTinted: { color: number; source: string };
  /** Canvas-level neutrals: background fill, shadow ink, and the
   *  high-contrast stroke colour used around icons / arrows. */
  canvas: {
    background: number;
    shadow: number;
    neutral: number;
  };
  /** Placeholder card sprite colours. */
  cardPlaceholder: {
    fill: number;
    stroke: number;
  };
  /** P/T badge background colours. */
  pt: {
    neutral: number;
    lethal: number;
    buffed: number;
    debuffed: number;
  };
  /** Per-counter-type chip colour. `default` is the fallback. */
  counter: {
    default: number;
    p1p1: number;
    m1m1: number;
    loyalty: number;
    charge: number;
    quest: number;
    study: number;
    lore: number;
    age: number;
    time: number;
    fade: number;
    level: number;
    storage: number;
    mining: number;
    brick: number;
    depletion: number;
    page: number;
  };
  cardRing: number;
  playerColors: {
    self: number;
    opponent1: number;
    opponent2: number;
    opponent3: number;
  };
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

  // Unparseable input — return a sentinel transparent value so we never
  // accidentally bake a hardcoded colour into the render.
  return { color: 0, alpha: 0 };
}

function parseHex(value: string): number {
  return parseColor(value).color;
}

export function adaptTheme(theme: GameThemeColors): PixiThemeColors {
  const pointer = {
    hostile: parseColor(theme.pointer.hostile),
    friendly: parseColor(theme.pointer.friendly),
  };

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
    pointer,
    mana: {
      W: parseHex(theme.mana.W),
      U: parseHex(theme.mana.U),
      B: parseHex(theme.mana.B),
      R: parseHex(theme.mana.R),
      G: parseHex(theme.mana.G),
      C: parseHex(theme.mana.C),
    },
    textOnTinted: { color: parseHex(theme.textOnTinted), source: theme.textOnTinted },
    canvas: {
      background: parseHex(theme.canvas.background),
      shadow: parseHex(theme.canvas.shadow),
      neutral: parseHex(theme.canvas.neutral),
    },
    cardPlaceholder: {
      fill: parseHex(theme.cardPlaceholder.fill),
      stroke: parseHex(theme.cardPlaceholder.stroke),
    },
    pt: {
      neutral: parseHex(theme.pt.neutral),
      lethal: parseHex(theme.pt.lethal),
      buffed: parseHex(theme.pt.buffed),
      debuffed: parseHex(theme.pt.debuffed),
    },
    cardStatus: {
      exerted: parseHex(theme.cardStatus.exerted),
      morph: parseHex(theme.cardStatus.morph),
      bestow: parseHex(theme.cardStatus.bestow),
      token: parseHex(theme.cardStatus.token),
      transformed: parseHex(theme.cardStatus.transformed),
      plotted: parseHex(theme.cardStatus.plotted),
      madness: parseHex(theme.cardStatus.madness),
      warped: parseHex(theme.cardStatus.warped),
    },
    counter: {
      default: parseHex(theme.counter.default),
      p1p1: parseHex(theme.counter.p1p1),
      m1m1: parseHex(theme.counter.m1m1),
      loyalty: parseHex(theme.counter.loyalty),
      charge: parseHex(theme.counter.charge),
      quest: parseHex(theme.counter.quest),
      study: parseHex(theme.counter.study),
      lore: parseHex(theme.counter.lore),
      age: parseHex(theme.counter.age),
      time: parseHex(theme.counter.time),
      fade: parseHex(theme.counter.fade),
      level: parseHex(theme.counter.level),
      storage: parseHex(theme.counter.storage),
      mining: parseHex(theme.counter.mining),
      brick: parseHex(theme.counter.brick),
      depletion: parseHex(theme.counter.depletion),
      page: parseHex(theme.counter.page),
    },
    cardRing: parseHex(theme.cardRing),
    playerColors: {
      self: parseHex(theme.playerColors.self),
      opponent1: parseHex(theme.playerColors.opponent1),
      opponent2: parseHex(theme.playerColors.opponent2),
      opponent3: parseHex(theme.playerColors.opponent3),
    },
  };
}
