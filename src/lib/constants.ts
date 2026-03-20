// ─── Routes ──────────────────────────────────────────────────────────────────

export const ROUTES = {
  LOBBY: "/lobby",
  PLAY: "/play",
  SEARCH: "/search",
  DECK_EDITOR: "/deck-editor",
  MY_DECKS: "/my-decks",
  SETTINGS: "/settings",
} as const;

// ─── Storage Keys ────────────────────────────────────────────────────────────

export const STORAGE_KEYS = {
  DECK: "xmage-deck-storage",
  PREFERRED_PRINTS: "xmage-preferred-prints",
  PREFERENCES: "xmage-preferences",
} as const;

// ─── Deck Defaults ───────────────────────────────────────────────────────────

export const DEFAULT_DECK_NAME = "New Deck";

// ─── Formats & Legalities ────────────────────────────────────────────────────

export const FORMAT_DISPLAY: Record<string, string> = {
  standard: "Standard",
  pioneer: "Pioneer",
  modern: "Modern",
  legacy: "Legacy",
  vintage: "Vintage",
  commander: "Commander",
  pauper: "Pauper",
  historic: "Historic",
  brawl: "Brawl",
};

export const LEGALITY_STYLES: Record<string, string> = {
  legal: "bg-green-500/20 text-green-400 border-green-500/30",
  banned: "bg-red-500/20 text-red-400 border-red-500/30",
  restricted: "bg-yellow-500/20 text-yellow-400 border-yellow-500/30",
  not_legal: "bg-muted text-muted-foreground border-border",
};

// ─── Drag & Drop IDs ────────────────────────────────────────────────────────

export const DROP_ZONE = {
  MAIN: "drop-main",
  SIDE: "drop-side",
  TAG_PREFIX: "drop-tag-",
} as const;

// ─── Set Types ──────────────────────────────────────────────────────────────

export const MAIN_SET_TYPES = new Set([
  "core",
  "expansion",
  "masters",
  "draft_innovation",
  "commander",
  "starter",
  "remaster",
]);
