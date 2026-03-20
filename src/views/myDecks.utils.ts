import type { Card } from "@/types/xmage";
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

// ── Card Categorization (Forge-style: Creatures, Spells, Lands) ─────────

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
