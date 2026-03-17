import type { Card } from "@/types/xmage";

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
    // Detect explicit colourless mana requirement {C}
    if (card.manaCost?.includes("{C}")) set.add("C");
  }
  return ["W", "U", "B", "R", "G", "C"].filter((c) => set.has(c));
}

// ── Card Grouping & Categorization ──────────────────────────────────────

export interface CardGroup {
  card: Card;
  count: number;
}

export function groupCards(cards: Card[]): CardGroup[] {
  const map = new Map<string, CardGroup>();
  for (const card of cards) {
    const existing = map.get(card.name);
    if (existing) existing.count++;
    else map.set(card.name, { card, count: 1 });
  }
  return Array.from(map.values()).sort((a, b) => {
    const aCmc = a.card.cmc ?? 0;
    const bCmc = b.card.cmc ?? 0;
    if (aCmc !== bCmc) return aCmc - bCmc;
    return a.card.name.localeCompare(b.card.name);
  });
}

// Group card list by type category (Forge-style: Lands, Creatures, Spells)
export function categorize(
  groups: CardGroup[],
): { label: string; items: CardGroup[] }[] {
  const lands: CardGroup[] = [];
  const creatures: CardGroup[] = [];
  const other: CardGroup[] = [];
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
