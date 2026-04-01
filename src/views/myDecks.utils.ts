import type { Card } from "@/types/openmagic";
import type { SavedDeck } from "@/stores/useDeckStore";
export { type CardGroup, groupCards } from "@/components/editor/deckBuilder.utils";

// ── Color Extraction ─────────────────────────────────────────────────────────

const VALID_COLORS = new Set(["W", "U", "B", "R", "G", "C"]);

export function extractColors(cards: Card[]): string[] {
  const set = new Set<string>();
  for (const card of cards) {
    for (const ch of card.color ?? "") {
      if (VALID_COLORS.has(ch)) set.add(ch);
    }
    if (card.manaCost?.includes("{C}")) set.add("C");
  }
  return ["W", "U", "B", "R", "G", "C"].filter((c) => set.has(c));
}

// ── Card Categorization (Forge-style: Creatures, Spells, Lands) ─────────────

export function categorize(
  groups: { card: Card; count: number }[],
): { label: string; items: { card: Card; count: number }[] }[] {
  const lands: typeof groups = [];
  const creatures: typeof groups = [];
  const other: typeof groups = [];
  for (const g of groups) {
    const types = g.card.types ?? [];
    if (types.includes("Land")) lands.push(g);
    else if (types.includes("Creature")) creatures.push(g);
    else other.push(g);
  }
  return [
    { label: "Creatures", items: creatures },
    { label: "Spells & Other", items: other },
    { label: "Lands", items: lands },
  ].filter((c) => c.items.length > 0);
}

// ── Filter / Sort ─────────────────────────────────────────────────────────────

export type SortBy = "name" | "color" | "updated";

export interface DeckFilters {
  search: string;
  formatFilter: string;
  colorFilter: string[];
  sortBy: SortBy;
}

/** Bitmask weight in WUBRG order — higher weight means "more white-leaning". */
const COLOR_BIT: Record<string, number> = { W: 16, U: 8, B: 4, R: 2, G: 1 };
function colorSortKey(colors: string[]): number {
  return colors.reduce((acc, c) => acc + (COLOR_BIT[c] ?? 0), 0);
}

/**
 * Filters and sorts `decks` according to `filters`, then splits the result
 * into non-draft (valid/complete) and draft (incomplete) sections.
 */
export function applyDeckFilters(
  decks: SavedDeck[],
  filters: DeckFilters,
): { valid: SavedDeck[]; drafts: SavedDeck[] } {
  const { search, formatFilter, colorFilter, sortBy } = filters;

  const pass = decks.filter((s) => {
    if (search && !s.deck.name.toLowerCase().includes(search.toLowerCase())) return false;
    if (formatFilter && (s.deck.format ?? "constructed") !== formatFilter) return false;
    if (colorFilter.length > 0) {
      const dc = extractColors(s.deck.cards);
      if (!colorFilter.every((c) => dc.includes(c))) return false;
    }
    return true;
  });

  const sortFn = (a: SavedDeck, b: SavedDeck): number => {
    switch (sortBy) {
      case "name":
        return a.deck.name.localeCompare(b.deck.name);
      case "color": {
        const ca = extractColors(a.deck.cards);
        const cb = extractColors(b.deck.cards);
        if (ca.length !== cb.length) return ca.length - cb.length;
        return colorSortKey(cb) - colorSortKey(ca);
      }
      case "updated":
        return b.savedAt - a.savedAt;
      default:
        return 0;
    }
  };

  return {
    valid: pass.filter((s) => !s.deck.draft).sort(sortFn),
    drafts: pass.filter((s) => !!s.deck.draft).sort(sortFn),
  };
}
