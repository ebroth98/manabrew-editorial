import type { Card } from "@/types/xmage";
import type { ScryfallCard } from "@/types/scryfall";
import { getScryfallImageUrl, getScryfallManaCost } from "@/api/scryfall";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface CardGroup {
  card: Card;
  count: number;
}

export type ViewMode = "list" | "visual" | "stack";

export interface SectionDefinition {
  id: string;
  label: string;
  filter: (types: string[]) => boolean;
}

// ─── Constants ────────────────────────────────────────────────────────────────

export const MAIN_SECTIONS: SectionDefinition[] = [
  { id: "creatures",     label: "Creatures",      filter: (t: string[]) => t.includes("Creature") },
  { id: "planeswalkers", label: "Planeswalkers",   filter: (t: string[]) => t.includes("Planeswalker") && !t.includes("Creature") },
  { id: "instants",      label: "Instants",        filter: (t: string[]) => t.includes("Instant") },
  { id: "sorceries",     label: "Sorceries",       filter: (t: string[]) => t.includes("Sorcery") },
  { id: "enchantments",  label: "Enchantments",    filter: (t: string[]) => t.includes("Enchantment") && !t.includes("Creature") },
  { id: "artifacts",     label: "Artifacts",       filter: (t: string[]) => t.includes("Artifact") && !t.includes("Creature") },
  { id: "lands",         label: "Lands",           filter: (t: string[]) => t.includes("Land") },
];

export const STACK_TYPE_COLS: SectionDefinition[] = [
  { id: "creatures",     label: "Creatures",     filter: (t: string[]) => t.includes("Creature") },
  { id: "instants",      label: "Instants",      filter: (t: string[]) => t.includes("Instant") },
  { id: "sorceries",     label: "Sorceries",     filter: (t: string[]) => t.includes("Sorcery") },
  { id: "enchantments",  label: "Enchantments",  filter: (t: string[]) => t.includes("Enchantment") && !t.includes("Creature") },
  { id: "artifacts",     label: "Artifacts",     filter: (t: string[]) => t.includes("Artifact") && !t.includes("Creature") },
  { id: "planeswalkers", label: "Planeswalkers", filter: (t: string[]) => t.includes("Planeswalker") && !t.includes("Creature") },
  { id: "lands",         label: "Lands",         filter: (t: string[]) => t.includes("Land") },
];

export const CARD_WIDTH_MAP: Record<number, number> = { 1: 75, 2: 95, 3: 115, 4: 140, 5: 170 };

export const GRID_COLS: Record<number, string> = {
  1: "grid-cols-5",
  2: "grid-cols-4",
  3: "grid-cols-3",
  4: "grid-cols-2",
  5: "grid-cols-1",
};

// ─── Pure Functions ───────────────────────────────────────────────────────────

/**
 * Groups cards by name, counting duplicates and sorting by CMC then name.
 */
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

/**
 * Converts a Scryfall card to a partial Card object with extracted properties.
 */
export function scryfallCardToPartial(sc: ScryfallCard): Partial<Card> {
  const SUPERTYPES = new Set(["Basic", "Legendary", "Snow", "World", "Ongoing"]);
  const [mainPart = "", subPart = ""] = sc.type_line.split("—").map((s) => s.trim());
  const mainTokens = mainPart.split(/\s+/).filter(Boolean);
  const supertypes = mainTokens.filter((t) => SUPERTYPES.has(t));
  const types = mainTokens.filter((t) => !SUPERTYPES.has(t));
  const subtypes = subPart ? subPart.split(/\s+/).filter(Boolean) : [];
  const imageUrl = getScryfallImageUrl(sc);
  const manaCost = getScryfallManaCost(sc) ?? "";
  return {
    manaCost, cmc: sc.cmc, types, subtypes, supertypes,
    color: (sc.colors ?? []).join(""),
    power: sc.power, toughness: sc.toughness,
    setCode: sc.set, cardNumber: sc.collector_number,
    ...(imageUrl ? { imageUrl } : {}),
  };
}

/**
 * Exports deck to Arena format (main deck + sideboard).
 */
export function exportToArena(deck: { name: string; cards: Card[]; sideboard: Card[] }): string {
  const mainGroups = groupCards(deck.cards);
  const sideGroups = groupCards(deck.sideboard);
  const lines: string[] = [];
  for (const g of mainGroups) lines.push(`${g.count} ${g.card.name}`);
  if (sideGroups.length > 0) {
    lines.push("");
    lines.push("Sideboard");
    for (const g of sideGroups) lines.push(`${g.count} ${g.card.name}`);
  }
  return lines.join("\n");
}

/**
 * Computes section groups for the main deck by filtering cards into type sections.
 */
export function computeSectionGroups(
  cards: Card[],
  sections: SectionDefinition[]
): Array<SectionDefinition & { groups: CardGroup[] }> {
  return sections.map((s) => ({
    ...s,
    groups: groupCards(cards.filter((c) => s.filter(c.types))),
  }));
}

/**
 * Computes "Other" group — cards that don't match any main section.
 */
export function computeOtherGroups(cards: Card[], sectionGroups: Array<{ groups: CardGroup[] }>): CardGroup[] {
  const matchedNames = new Set(sectionGroups.flatMap((s) => s.groups.map((g) => g.card.name)));
  return groupCards(cards.filter((c) => !matchedNames.has(c.name)));
}

/**
 * Computes stack-mode columns by grouping cards into type columns (no overlap).
 */
export function computeStackColumns(
  cards: Card[],
  columns: SectionDefinition[]
): Array<SectionDefinition & { groups: CardGroup[] }> {
  const allGroups = groupCards(cards);
  const usedNames = new Set<string>();
  const cols = columns.map((col) => ({
    ...col,
    groups: allGroups.filter((g) => {
      if (usedNames.has(g.card.name)) return false;
      if (col.filter(g.card.types)) { usedNames.add(g.card.name); return true; }
      return false;
    }),
  }));
  const otherGroups = allGroups.filter((g) => !usedNames.has(g.card.name));
  if (otherGroups.length > 0) cols.push({ id: "other", label: "Other", filter: () => false, groups: otherGroups });
  return cols.filter((c) => c.groups.length > 0);
}
