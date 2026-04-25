/**
 * Shared in-memory cache of Scryfall `image_uris` records, keyed by card
 * identity (name + set_code + card_number + token flag). Both the React
 * `useCardImage` hook and the Pixi texture loader read / write this cache,
 * so any component that triggers a fetch (typically the hand sprite for a
 * newly-drawn card) "warms" the entry for every later consumer — the
 * battlefield, the stack, the zone modals, hover previews — without a
 * second network call per size variant.
 *
 * Scryfall returns all size variants (`small`, `normal`, `large`, `png`)
 * in one response, so caching the whole record (plus DFC face records)
 * lets any caller resolve its preferred size from the same entry.
 */
import type { ScryfallCard, ScryfallImageUris } from "@/types/scryfall";

export interface CachedCardImage {
  /** Top-level `image_uris` — present for single-faced cards. */
  imageUris?: ScryfallImageUris;
  /** Per-face name (lowercased) → face-specific `image_uris`. Present for
   *  double-faced cards; empty for single-faced. */
  faces: Map<string, ScryfallImageUris>;
}

const cache = new Map<string, CachedCardImage>();
/** In-flight fetches by identity so concurrent consumers share the promise. */
const pending = new Map<string, Promise<CachedCardImage | null>>();

/** Discriminators for the two Scryfall lookup kinds baked into the
 *  cache key. Kept as named constants rather than raw strings so any
 *  future addition (e.g. commander, planar card) has an obvious home. */
const IDENTITY_KIND_CARD = "card";
const IDENTITY_KIND_TOKEN = "token";

export function cardImageIdentity(
  name: string,
  setCode: string | undefined,
  cardNumber: string | undefined,
  isToken: boolean | undefined,
): string {
  return [
    isToken ? IDENTITY_KIND_TOKEN : IDENTITY_KIND_CARD,
    name.toLowerCase(),
    (setCode ?? "").toLowerCase(),
    cardNumber ?? "",
  ].join("::");
}

export function getCachedCardImage(key: string): CachedCardImage | undefined {
  return cache.get(key);
}

export function setCachedCardImage(key: string, value: CachedCardImage): void {
  cache.set(key, value);
}

export function getPendingCardImage(
  key: string,
): Promise<CachedCardImage | null> | undefined {
  return pending.get(key);
}

export function setPendingCardImage(
  key: string,
  p: Promise<CachedCardImage | null>,
): void {
  pending.set(key, p);
}

export function clearPendingCardImage(key: string): void {
  pending.delete(key);
}

/**
 * Extract the cacheable view of a ScryfallCard: the top-level image_uris
 * plus the (optional) per-face uris for double-faced cards.
 */
export function toCachedCardImage(card: ScryfallCard, { frontOnly }: { frontOnly: boolean}): CachedCardImage {
  const entry: CachedCardImage = { faces: new Map() };

  const top = card.image_uris;
  if (top) entry.imageUris = top;
  const faces = card.card_faces;
  if (faces) {
    for (const f of faces) {
      if (f.name && f.image_uris && (!frontOnly || f.image_uris.small.includes("/front/"))) {
        entry.faces.set(f.name.toLowerCase(), f.image_uris);
      }
    }
  }
  return entry;
}

/**
 * Given a cached entry and the name the caller wants (which may be the
 * front or back face of a DFC), pick the best-matching image_uris block.
 */
export function pickUrisForName(
  entry: CachedCardImage,
  name: string,
): ScryfallImageUris | undefined {
  const face = entry.faces.get(name.toLowerCase()) ?? entry.faces.values().next().value;
  if (face) return face;
  return entry.imageUris;
}

export type ScryfallImageSize = "small" | "normal" | "large" | "png";

/** Return the best available URL from a uris block, preferring the
 *  requested size and falling back through larger variants. */
export function pickImageUrl(
  uris: ScryfallImageUris | undefined,
  size: ScryfallImageSize = "normal",
): string | undefined {
  if (!uris) return undefined;
  return uris[size] ?? uris.normal ?? uris.large ?? uris.png;
}
