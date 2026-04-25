import type { ScryfallCard, ScryfallListResponse, ScryfallRulingsResponse, ScryfallSet } from "@/types/scryfall";

const SCRYFALL_API = "https://api.scryfall.com";
const COLLECTION_BATCH_SIZE = 75;
const SCRYFALL_QUEUE_DELAY_MS = 120;
const SCRYFALL_MAX_RETRIES = 3;

const scryfallQueue: Array<() => Promise<void>> = [];
let scryfallQueueRunning = false;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function parseRetryAfterMs(retryAfter: string | null): number | null {
  if (!retryAfter) return null;
  const seconds = Number(retryAfter);
  if (Number.isFinite(seconds) && seconds >= 0) {
    return Math.ceil(seconds * 1000);
  }
  const retryDate = Date.parse(retryAfter);
  if (Number.isNaN(retryDate)) return null;
  return Math.max(retryDate - Date.now(), 0);
}

async function drainScryfallQueue(): Promise<void> {
  if (scryfallQueueRunning) return;
  scryfallQueueRunning = true;
  while (scryfallQueue.length > 0) {
    const job = scryfallQueue.shift()!;
    await job();
    if (scryfallQueue.length > 0) {
      await sleep(SCRYFALL_QUEUE_DELAY_MS);
    }
  }
  scryfallQueueRunning = false;
}

function queueScryfallCall<T>(fn: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    scryfallQueue.push(async () => {
      try {
        resolve(await fn());
      } catch (error) {
        reject(error);
      }
    });
    drainScryfallQueue();
  });
}

export function scryfallCardKey(name: string, setCode?: string): string {
  return setCode ? `${name.toLowerCase()}::${setCode.toLowerCase()}` : name.toLowerCase();
}

async function performScryfallFetch<T>(url: string, errorMsg: string, init?: RequestInit): Promise<T> {
  for (let attempt = 0; attempt <= SCRYFALL_MAX_RETRIES; attempt += 1) {
    const response = await fetch(url, init);
    if (response.status === 429 && attempt < SCRYFALL_MAX_RETRIES) {
      const retryAfterMs = parseRetryAfterMs(response.headers.get("retry-after"));
      await sleep(retryAfterMs ?? 1000 * (attempt + 1));
      continue;
    }
    if (!response.ok) {
      throw new Error(`${errorMsg} (HTTP ${response.status})`);
    }
    return response.json();
  }
  throw new Error(errorMsg);
}

async function scryfallFetch<T>(url: string, errorMsg: string, init?: RequestInit): Promise<T> {
  return queueScryfallCall(() => performScryfallFetch<T>(url, errorMsg, init));
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
 * Fetch a token card from Scryfall using its set code and collector number.
 * The backend resolves these from Forge edition files — each token has a
 * dedicated Scryfall token set (e.g., "thou" for Tokens of Hour of Devastation)
 * and a collector number within that set.
 *
 * Uses the /cards/:set/:number endpoint which is a direct, unambiguous lookup.
 */
export async function getTokenBySetAndNumber(setCode: string, collectorNumber: string): Promise<ScryfallCard> {
  const tokenUrl = `${SCRYFALL_API}/cards/${encodeURIComponent(setCode.toLowerCase())}/${encodeURIComponent(collectorNumber)}`
  console.log({tokenUrl})
  return scryfallFetch(tokenUrl,`Token not found: ${setCode}/${collectorNumber}`);
}

/**
 * Search Scryfall for a token print by name. Always restricts the query
 * to `type:token` so we don't accidentally return the legendary creature
 * with the same name (e.g. "Goblin" would otherwise hit Goblin cards
 * from the base set before the actual Goblin token print).
 *
 * Returns the first matching print, preferring the newest release, or
 * null when no token with that exact name exists on Scryfall. Used as
 * the fallback image resolver when the engine didn't supply a
 * `set_code` + `collector_number` pair for a token (e.g. because the
 * token's edition file has no `[tokens]` section).
 */
export async function getTokenByName(name: string): Promise<ScryfallCard | null> {
  const query = `!"${name}" type:token`;
  const url = `${SCRYFALL_API}/cards/search?q=${encodeURIComponent(query)}&unique=prints&order=released&dir=desc`;
  try {
    const data = await scryfallFetch<{ data?: ScryfallCard[] }>(
      url,
      `No token print found for name: ${name}`,
    );
    return data.data?.[0] ?? null;
  } catch {
    return null;
  }
}

/**
 * Batch-fetch cards by name using POST /cards/collection (up to 75 per request).
 * Returns a map keyed by lowercased card name → ScryfallCard.
 */
export async function fetchCardCollection(cards: { name: string; setCode?: string }[]): Promise<Map<string, ScryfallCard>> {
  const result = new Map<string, ScryfallCard>();
  const unique = Array.from(new Map(cards.map((c) => [scryfallCardKey(c.name, c.setCode), c])).values());
  for (let i = 0; i < unique.length; i += COLLECTION_BATCH_SIZE) {
    const batch = unique.slice(i, i + COLLECTION_BATCH_SIZE);
    const identifiers = batch.map((c) => (c.setCode ? { name: c.name, set: c.setCode.toLowerCase() } : { name: c.name }));
    try {
      const data = await scryfallFetch<{ data: ScryfallCard[]; not_found: { name: string }[] }>(
        `${SCRYFALL_API}/cards/collection`,
        "Failed to fetch card collection from Scryfall",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ identifiers }),
        },
      );
      for (const card of data.data) {
        const setAwareKey = scryfallCardKey(card.name, card.set);
        const legacyKey = scryfallCardKey(card.name);
        result.set(setAwareKey, card);
        if (!result.has(legacyKey)) {
          result.set(legacyKey, card);
        }
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
export function getScryfallImageUrl(card: ScryfallCard, size: string = "normal"): string | undefined {
  const sc = card as unknown as {
    card_faces?: { image_uris?: Record<string, string> }[];
    image_uris?: Record<string, string>;
  };
  const uris = sc.image_uris ?? sc.card_faces?.[0]?.image_uris;
  return uris?.[size] ?? uris?.normal ?? uris?.large;
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
