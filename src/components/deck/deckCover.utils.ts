import type { Card, Deck } from "@/types/openmagic";

export function resolveCoverCard(deck: Deck): Card {
  const allCards = [...deck.cards, ...(deck.commanders ?? [])];
  if (deck.coverCardName) {
    const found = allCards.find((card) => card.name === deck.coverCardName);
    if (found) return found;
  }
  return deck.commanders?.[0] ?? deck.cards[0];
}

export const resolvePresetDeck = (presetDeck: Deck): Card => resolveCoverCard(presetDeck);
