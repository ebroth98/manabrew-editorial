import type { Card, Deck } from "@/types/manabrew";
import type { CardIdentity, DeckSection } from "@/types/server";

export function deckSectionForCard(card: Card, fallback: DeckSection): DeckSection {
  if (card.subtypes.some((subtype) => subtype.toLowerCase() === "attraction")) {
    return "attractions";
  }
  if (card.subtypes.some((subtype) => subtype.toLowerCase() === "contraption")) {
    return "contraptions";
  }
  if (card.types.some((type) => type.toLowerCase() === "scheme")) {
    return "schemes";
  }
  if (card.types.some((type) => type.toLowerCase() === "plane")) {
    return "planes";
  }
  return fallback;
}

export function toCardIdentity(card: Card, section: DeckSection): CardIdentity {
  return {
    name: card.name,
    setCode: card.setCode || "",
    section: deckSectionForCard(card, section),
    foil: card.foil ? true : undefined,
  };
}

export function serializeDeck(deck: Deck): CardIdentity[] {
  return [
    ...deck.cards.map((card) => toCardIdentity(card, "main")),
    ...deck.sideboard.map((card) => toCardIdentity(card, "sideboard")),
    ...(deck.attractions ?? []).map((card) => toCardIdentity(card, "attractions")),
    ...(deck.contraptions ?? []).map((card) => toCardIdentity(card, "contraptions")),
    ...(deck.schemes ?? []).map((card) => toCardIdentity(card, "schemes")),
    ...(deck.planes ?? []).map((card) => toCardIdentity(card, "planes")),
    ...(deck.commanders ?? []).map((card) => toCardIdentity(card, "commander")),
  ];
}

export function getDeckCardPool(deck: Deck): Card[] {
  return [
    ...deck.cards,
    ...deck.sideboard,
    ...(deck.attractions ?? []),
    ...(deck.contraptions ?? []),
    ...(deck.schemes ?? []),
    ...(deck.planes ?? []),
    ...(deck.commanders ?? []),
  ];
}

export function getDeckCardNames(deck: Deck): string[] {
  return serializeDeck(deck)
    .filter((card) => card.section === "main" || card.section === "commander")
    .map((card) => card.name);
}

export function getDeckFingerprint(deck: Deck): string {
  const serialized = serializeDeck(deck)
    .map((card) => `${card.section ?? "main"}:${card.name}:${card.setCode}`)
    .sort();
  return JSON.stringify({
    name: deck.name,
    format: deck.format ?? "standard",
    commander: deck.commanders?.[0]?.name ?? null,
    cards: serialized,
  });
}
