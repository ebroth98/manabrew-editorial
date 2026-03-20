import type { ScryfallCard, ScryfallListResponse, ScryfallRulingsResponse, ScryfallSet } from "@/types/scryfall";

const SCRYFALL_API = "https://api.scryfall.com";
const COLLECTION_BATCH_SIZE = 75;

async function scryfallFetch<T>(url: string, errorMsg: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, init);
  if (!response.ok) throw new Error(errorMsg);
  return response.json();
}

export async function searchCards(query: string, page: number = 1, order?: string, dir?: string): Promise<ScryfallListResponse> {
  const orderParam = order || "cmc";
  const dirParam = dir && dir !== "auto" ? `&dir=${dir}` : "";
  return scryfallFetch(
    `${SCRYFALL_API}/cards/search?q=${encodeURIComponent(query)}&page=${page}&order=${orderParam}&unique=cards${dirParam}`,
    "Failed to fetch cards from Scryfall",
  );
}

export async function getRulings(rulingsUri: string): Promise<ScryfallRulingsResponse> {
  return scryfallFetch(rulingsUri, "Failed to fetch rulings from Scryfall");
}

export async function getCardPrints(printsSearchUri: string): Promise<ScryfallListResponse> {
  return scryfallFetch(printsSearchUri, "Failed to fetch card prints from Scryfall");
}

export async function getCardByName(name: string, setCode?: string): Promise<ScryfallCard> {
  const setParam = setCode ? `&set=${setCode.toLowerCase()}` : "";
  const url = `${SCRYFALL_API}/cards/named?exact=${encodeURIComponent(name)}${setParam}`;
  try {
    return await scryfallFetch<ScryfallCard>(url, `Card not found: ${name}`);
  } catch {
    if (setCode) return getCardByName(name);
    throw new Error(`Card not found: ${name}`);
  }
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
  const url = `${SCRYFALL_API}/cards/search?q=!"${encodeURIComponent(searchName)}"+t:token${colorQuery}+-is:universesbeyond&order=released&dir=asc&unique=art`;
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
  for (let i = 0; i < unique.length; i += COLLECTION_BATCH_SIZE) {
    const batch = unique.slice(i, i + COLLECTION_BATCH_SIZE);
    const identifiers = batch.map((c) => (c.setCode ? { name: c.name, set: c.setCode.toLowerCase() } : { name: c.name }));
    try {
      const response = await fetch(`${SCRYFALL_API}/cards/collection`, {
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

/**
 * Extract the primary image URL from a Scryfall card response.
 * Handles both single-faced cards (top-level image_uris) and double-faced cards
 * (image_uris in card_faces array).
 */
export function getScryfallImageUrl(card: ScryfallCard): string | undefined {
  const sc = card as unknown as { 
    card_faces?: { image_uris?: { normal?: string } }[];
    image_uris?: { normal?: string };
  };
  return sc.image_uris?.normal ?? sc.card_faces?.[0]?.image_uris?.normal;
}

/**
 * Extract mana cost from a Scryfall card (handles DFCs).
 * For double-faced cards, returns the front face's mana cost.
 */
export function getScryfallManaCost(card: ScryfallCard): string | undefined {
  const sc = card as unknown as {
    card_faces?: { mana_cost?: string }[];
    mana_cost?: string;
  };
  return sc.mana_cost ?? sc.card_faces?.[0]?.mana_cost;
}

/**
 * Fetch all Magic sets from Scryfall.
 */
export async function fetchSets(): Promise<ScryfallSet[]> {
  const data = await scryfallFetch<{ data: ScryfallSet[] }>(
    `${SCRYFALL_API}/sets`,
    "Failed to fetch sets from Scryfall",
  );
  return data.data;
}

/** Build a Scryfall mana symbol SVG URL. */
export function manaSymbolUrl(symbol: string): string {
  return `https://svgs.scryfall.io/card-symbols/${encodeURIComponent(symbol)}.svg`;
}
