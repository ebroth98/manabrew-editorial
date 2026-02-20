import type { ScryfallCard, ScryfallListResponse } from "@/types/scryfall";

export async function searchCards(query: string, page: number = 1): Promise<ScryfallListResponse> {
  const url = `https://api.scryfall.com/cards/search?q=${encodeURIComponent(query)}&page=${page}&order=cmc&unique=cards`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error('Failed to fetch cards from Scryfall');
  }
  return response.json();
}

export async function getCardByName(name: string): Promise<ScryfallCard> {
  const url = `https://api.scryfall.com/cards/named?exact=${encodeURIComponent(name)}`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Card not found: ${name}`);
  }
  return response.json();
}

/**
 * Batch-fetch cards by name using POST /cards/collection (up to 75 per request).
 * Returns a map keyed by lowercased card name → ScryfallCard.
 */
export async function fetchCardCollection(names: string[]): Promise<Map<string, ScryfallCard>> {
  const result = new Map<string, ScryfallCard>();
  const unique = [...new Set(names)];
  for (let i = 0; i < unique.length; i += 75) {
    const batch = unique.slice(i, i + 75);
    const identifiers = batch.map((name) => ({ name }));
    try {
      const response = await fetch("https://api.scryfall.com/cards/collection", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ identifiers }),
      });
      if (!response.ok) continue;
      const data: { data: ScryfallCard[]; not_found: { name: string }[] } = await response.json();
      for (const card of data.data) {
        result.set(card.name.toLowerCase(), card);
      }
    } catch {
      // best-effort per batch
    }
  }
  return result;
}
