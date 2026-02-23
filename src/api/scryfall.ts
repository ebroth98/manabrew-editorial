import type { ScryfallCard, ScryfallListResponse } from "@/types/scryfall";

export async function searchCards(query: string, page: number = 1): Promise<ScryfallListResponse> {
  const url = `https://api.scryfall.com/cards/search?q=${encodeURIComponent(query)}&page=${page}&order=cmc&unique=cards`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error('Failed to fetch cards from Scryfall');
  }
  return response.json();
}

export async function getCardPrints(printsSearchUri: string): Promise<ScryfallListResponse> {
  const response = await fetch(printsSearchUri);
  if (!response.ok) {
    throw new Error('Failed to fetch card prints from Scryfall');
  }
  return response.json();
}

export async function getCardByName(name: string, setCode?: string): Promise<ScryfallCard> {
  const url = `https://api.scryfall.com/cards/named?exact=${encodeURIComponent(name)}${setCode ? `&set=${setCode.toLowerCase()}` : ""}`;
  const response = await fetch(url);
  if (!response.ok) {
    if (setCode) {
      return getCardByName(name);
    }
    throw new Error(`Card not found: ${name}`);
  }
  return response.json();
}
  

/**
 * Convert an engine ColorSet string (e.g. "W", "WU", "C") to Scryfall color
 * filter tokens (e.g. ["c:w", "c:u"]). Returns empty array for colorless/unknown.
 */
function colorFilters(engineColor: string): string[] {
  if (!engineColor || engineColor === "C") return [];
  return engineColor
    .toUpperCase()
    .split("")
    .filter((ch) => "WUBRG".includes(ch))
    .map((ch) => `c:${ch.toLowerCase()}`);
}

/**
 * Find a token card on Scryfall by name and color.
 * Forge token scripts append " Token" to the name (e.g. "Goblin Token"),
 * but Scryfall names tokens without that suffix (e.g. "Goblin").
 * The color filter (engine format: "W", "WU", "C") disambiguates tokens that
 * share a name but differ in color (e.g. white vs red Soldier tokens).
 * Returns the oldest classic MTG printing to avoid crossover/themed set art.
 */
export async function getTokenByName(name: string, color?: string): Promise<ScryfallCard> {
  // Strip trailing " Token" added by Forge token script naming convention
  const searchName = name.endsWith(" Token") ? name.slice(0, -6) : name;
  const colorPart = color ? colorFilters(color).join("+") : "";
  const colorQuery = colorPart ? `+${colorPart}` : "";
  // dir=asc → oldest first, so data[0] is the classic original art instead of
  // the newest crossover/themed printing. unique=art → one result per artwork.
  const url = `https://api.scryfall.com/cards/search?q=!"${encodeURIComponent(searchName)}"+t:token${colorQuery}+-is:universesbeyond&order=released&dir=asc&unique=art`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Token not found: ${name}`);
  }
  const data: ScryfallListResponse = await response.json();
  if (!data.data.length) {
    throw new Error(`Token not found: ${name}`);
  }
  return data.data[0];
}

/**
 * Batch-fetch cards by name using POST /cards/collection (up to 75 per request).
 * Returns a map keyed by lowercased card name → ScryfallCard.
 */
export async function fetchCardCollection(cards: { name: string; setCode?: string }[]): Promise<Map<string, ScryfallCard>> {
  const result = new Map<string, ScryfallCard>();
  const unique = Array.from(new Map(cards.map((c) => [`${c.name}-${c.setCode || ""}`, c])).values());
  for (let i = 0; i < unique.length; i += 75) {
    const batch = unique.slice(i, i + 75);
    const identifiers = batch.map((c) => (c.setCode ? { name: c.name, set: c.setCode.toLowerCase() } : { name: c.name }));
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
