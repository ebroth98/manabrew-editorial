import { useQuery } from "@tanstack/react-query";
import { getCardByName, getTokenByName, getTokenBySetAndNumber } from "@/api/scryfall";
import type { ScryfallCard } from "@/types/scryfall";
import {
  cardImageIdentity,
  clearPendingCardImage,
  getCachedCardImage,
  getPendingCardImage,
  pickImageUrl,
  pickUrisForName,
  setCachedCardImage,
  setPendingCardImage,
  toCachedCardImage,
  type CachedCardImage,
  type ScryfallImageSize,
} from "@/lib/cardImageCache";

const CARD_IMAGE_FETCH_DEBOUNCE_MS = 80;

/**
 * Resolve (and cache) the full `image_uris` record for a card identity.
 * Every consumer — the hand sprite, battlefield grid, stack panel,
 * modals, hover preview, Pixi texture loader — hits the same identity
 * key, so the *first* caller to see a given (name, setCode, cardNumber,
 * isToken) tuple pays for the API call and everyone after reads from
 * the cache synchronously. This is the "load once in the hand, pass it
 * everywhere else" contract the UI needs.
 */
export async function fetchCardImageEntry(
  name: string,
  isToken: boolean | undefined,
  setCode: string | undefined,
  cardNumber: string | undefined,
): Promise<CachedCardImage | null> {
  const key = cardImageIdentity(name, setCode, cardNumber, isToken);
  const cached = getCachedCardImage(key);
  if (cached) return cached;
  const inflight = getPendingCardImage(key);
  if (inflight) return inflight;

  const task = (async (): Promise<CachedCardImage | null> => {
    await new Promise((resolve) => setTimeout(resolve, CARD_IMAGE_FETCH_DEBOUNCE_MS));
    let card: ScryfallCard | null = null;
    if (isToken) {
      if (setCode && cardNumber) {
        try {
          card = await getTokenBySetAndNumber(setCode, cardNumber);
        } catch {
          card = null;
        }
      }
      if (!card) card = await getTokenByName(name);
      if (!card) {
        try {
          card = await getCardByName(name, setCode);
        } catch {
          card = null;
        }
      }
    } else {
      try {
        card = await getCardByName(name, setCode);
      } catch {
        card = null;
      }
    }
    if (!card) return null;
    const entry = toCachedCardImage(card, { frontOnly: !!isToken });
    setCachedCardImage(key, entry);
    return entry;
  })().finally(() => clearPendingCardImage(key));
  setPendingCardImage(key, task);
  return task;
}

/**
 * Returns a Scryfall image URL for the given card name.
 *
 * - If the card already has an `existingUrl` (seeded on the DTO), skips
 *   the fetch entirely.
 * - Otherwise consults the shared cache (populated by any previous
 *   component that resolved this identity) before falling through to a
 *   network call. A single cache entry serves every requested size.
 */
export function useCardImage(
  name: string,
  existingUrl?: string,
  isToken?: boolean,
  _color?: string,
  setCode?: string,
  cardNumber?: string,
  size: ScryfallImageSize = "normal",
) {
  const identity = cardImageIdentity(name, setCode, cardNumber, isToken);
  // `size` is intentionally *not* part of the query key. One API call
  // yields every size variant (Scryfall returns the full image_uris
  // block), so we cache the block once per identity and derive the
  // requested size via `select` below.
  return useQuery({
    queryKey: ["card-image-uris", identity],
    queryFn: () => fetchCardImageEntry(name, isToken, setCode, cardNumber),
    select: (entry) => {
      if (!entry) return undefined;
      return pickImageUrl(pickUrisForName(entry, name), size);
    },
    enabled: !!name && !existingUrl,
    staleTime: Infinity, // card images never change
    gcTime: 1000 * 60 * 60, // keep in cache 1 hour
    retry: false,
    // `initialData` lets the hook return the cached entry synchronously
    // on first render when another consumer has already resolved this
    // identity. Without it the UI would briefly render a loader even
    // though the URL is already in memory.
    initialData: () => getCachedCardImage(identity) ?? undefined,
  });
}
