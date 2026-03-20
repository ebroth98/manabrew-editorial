import type { Card } from "@/types/xmage";
export { type CardGroup, groupCards } from "@/components/editor/deckBuilder.utils";

// ── Color Extraction & Constants ────────────────────────────────────────

export const COLOR_MAP: Record<string, { bg: string; border: string; label: string }> = {
  W: { bg: "bg-yellow-50", border: "border-yellow-300", label: "W" },
  U: { bg: "bg-blue-100", border: "border-blue-400", label: "U" },
  B: { bg: "bg-gray-800", border: "border-gray-600", label: "B" },
  R: { bg: "bg-red-100", border: "border-red-400", label: "R" },
  G: { bg: "bg-green-100", border: "border-green-400", label: "G" },
  C: { bg: "bg-zinc-200", border: "border-zinc-400", label: "C" },
};

export function extractColors(cards: Card[]): string[] {
  const set = new Set<string>();
  for (const card of cards) {
    for (const ch of card.color ?? "") {
      if (ch in COLOR_MAP) set.add(ch);
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
