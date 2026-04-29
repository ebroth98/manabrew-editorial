import { create } from "zustand";
import { devtools } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";
import {
  fetchCardsBySet,
  fetchImageElement,
  fetchSets,
  getCardById,
  getCardByName,
  getCardBySetAndNumber,
  getRulings,
} from "@/api/scryfall";
import type { ScryfallCard, ScryfallRulingsResponse, ScryfallSet } from "@/types/scryfall";
import { Texture, ImageSource } from "pixi.js";
import { useEffect, useState } from "react";

export interface ScryfallCardLookup {
  id?: string;
  name?: string;
  setCode?: string;
  collectorNumber?: string;
}

type CardEntry = {
  info: ScryfallCard;
  texture: Texture;
  uris: NonNullable<ScryfallCard["image_uris"]>;
};

interface ScryfallEntry {
  card?: CardEntry;
  pendingPromise?: Promise<CardEntry>;
}

interface ScryfallState {
  _fetchCardLookup: (lookup: ScryfallCardLookup) => Promise<CardEntry>;
  cards: Record<string, ScryfallEntry>;
  sets: ScryfallSet[];
  /** Lowercased set codes whose card metadata has already been hydrated.
   *  Object-shaped (not `Set`) so immer can produce drafts without the
   *  MapSet plugin. */
  hydratedSets: Record<string, true>;
  getCard: (lookup: ScryfallCardLookup) => Promise<CardEntry>;
  getCardTexture: (lookup: ScryfallCardLookup) => Promise<CardEntry>;
  updatePrinting: (card: ScryfallCard) => CardEntry;
  invalidateCard: (name: string) => void;
  getRulings: (card: { rulings_uri: string }) => Promise<ScryfallRulingsResponse>;
  /**
   * Fetch every printing in `setCode` via Scryfall's search endpoint
   * and prime the cache under all the lookup keys (`set:..::cn:..`,
   * `name:..`, `name:..::set:..`, `id:..`). Subsequent `useCard` calls
   * for any of those resolve from cache instead of firing per-card
   * lookups.
   */
  hydrateSet: (setCode: string) => Promise<ScryfallCard[]>;
  /**
   * Like `hydrateSet`, but also kicks off image-texture downloads for
   * every card so a follow-up draft / sealed open is fully warm.
   * Returns once metadata is in cache; image fetches continue in the
   * background.
   */
  prefetchSet: (setCode: string) => Promise<void>;
}

function scryfallLookupKey({ id, name, setCode, collectorNumber }: ScryfallCardLookup): string {
  if (id) return `id:${id}`;
  const normalizedSet = setCode?.toLowerCase();
  if (normalizedSet && collectorNumber) {
    return `set:${normalizedSet}::cn:${collectorNumber.toLowerCase()}`;
  }
  if (!name) {
    throw new Error("Scryfall lookup requires a name or setCode + collectorNumber");
  }
  return normalizedSet
    ? `name:${name.toLowerCase()}::set:${normalizedSet}`
    : `name:${name.toLowerCase()}`;
}

async function fetchScryfallCard(lookup: ScryfallCardLookup): Promise<ScryfallCard> {
  if (lookup.id) {
    return getCardById(lookup.id);
  }
  if (lookup.setCode && lookup.collectorNumber) {
    return getCardBySetAndNumber(lookup.setCode, lookup.collectorNumber);
  }
  if (!lookup.name) {
    throw new Error("Scryfall lookup requires a name or id");
  }
  return getCardByName(lookup.name, lookup.setCode);
}

export const chooseImageUrisForCard = (
  info: ScryfallCard,
  { frontOnly }: { frontOnly: boolean },
) => {
  if (info.image_uris) {
    return info.image_uris; // TODO: which one?
  }
  if (info.card_faces) {
    for (const f of info.card_faces) {
      if (f.name && f.image_uris && (!frontOnly || f.image_uris.small.includes("/front/"))) {
        return f.image_uris;
      }
    }
  }
  return null; //TODO:
};

const createTextureFromImage = (img: HTMLImageElement): Texture => {
  const source = new ImageSource({ resource: img });
  const tex = new Texture({ source });
  return tex;
};

const pendingTexturePromises = new Map<string, Promise<CardEntry>>();

/**
 * Cache key shapes the resolved entry should be mirrored under so any
 * subsequent caller — querying by id, set+collector, name+set, or just
 * name — hits the same Scryfall printing instead of re-fetching and
 * potentially resolving a different default printing.
 *
 * Pure (no draft / immer types) — the actual writes are inlined at the
 * call site inside the immer producer where draft assignments are
 * native and need no cast.
 */
function siblingKeysFor(entry: ScryfallEntry): string[] {
  const info = entry.card?.info;
  if (!info) return [];
  const keys: string[] = [];
  const lowerName = info.name?.toLowerCase();
  const setCode = info.set?.toLowerCase();
  const cn = info.collector_number?.toLowerCase();
  if (info.id) keys.push(`id:${info.id}`);
  if (setCode && cn) keys.push(`set:${setCode}::cn:${cn}`);
  if (lowerName && setCode) keys.push(`name:${lowerName}::set:${setCode}`);
  if (lowerName) keys.push(`name:${lowerName}`);
  return keys;
}

export const useScryfallStore = create<ScryfallState>()(
  devtools(
    immer((set, get) => ({
      cards: {},
      hydratedSets: {},
      _fetchCardLookup: async (lookup) => {
        const key = scryfallLookupKey(lookup);
        console.log("===== ACTUALLY FETCHING: " + key);
        const card = await fetchScryfallCard(lookup);

        const uris = chooseImageUrisForCard(card, { frontOnly: true });
        if (!uris) {
          throw new Error("Couldn't find a texture url for: " + JSON.stringify(lookup));
        }

        const entry: ScryfallEntry = {
          card: { info: card, texture: Texture.EMPTY, uris },
        };
        const mirrorKeys = siblingKeysFor(entry);
        const newId = entry.card?.info?.id;
        set((state) => {
          state.cards[key] = entry;
          for (const k of mirrorKeys) {
            // Preserve pinnings (e.g. from `updatePrinting`) by only
            // overwriting empty slots or slots already pointing at the
            // same Scryfall printing.
            const existingId = state.cards[k]?.card?.info?.id;
            if (existingId == null || existingId === newId) state.cards[k] = entry;
          }
        });
        return entry.card!;
      },
      getCard: async (lookup) => {
        const key = scryfallLookupKey(lookup);
        const existing = get().cards[key];
        if (existing?.card) return existing.card;
        if (existing?.pendingPromise) return existing.pendingPromise;

        const { _fetchCardLookup } = get();
        const pendingPromise = _fetchCardLookup(lookup);
        set((state) => {
          state.cards[key] = { pendingPromise };
        });
        return pendingPromise;
      },
      getCardTexture: async (lookup) => {
        const key = scryfallLookupKey(lookup);
        const existing = get().cards[key];
        if (existing?.card && existing.card.texture !== Texture.EMPTY) return existing.card;

        const pendingTexture = pendingTexturePromises.get(key);
        if (pendingTexture) return pendingTexture;

        const pendingTexturePromise = (async () => {
          const card = await get().getCard(lookup);
          if (card.texture !== Texture.EMPTY) {
            return card;
          }

          const htmlImage = await fetchImageElement(card.uris.border_crop);
          const texture = createTextureFromImage(htmlImage);
          const entry = { ...card, texture };
          const wrapper: ScryfallEntry = { card: entry };
          const mirrorKeys = siblingKeysFor(wrapper);
          const newId = entry.info?.id;
          set((state) => {
            state.cards[key] = wrapper;
            for (const k of mirrorKeys) {
              const existingId = state.cards[k]?.card?.info?.id;
              if (existingId == null || existingId === newId) state.cards[k] = wrapper;
            }
          });
          return entry;
        })().finally(() => {
          pendingTexturePromises.delete(key);
        });
        pendingTexturePromises.set(key, pendingTexturePromise);
        return pendingTexturePromise;
      },
      getRulings: async (c) => {
        const rulingsUri = c.rulings_uri;
        return getRulings(rulingsUri);
      },
      hydrateSet: async (setCode) => {
        const code = setCode.toLowerCase();
        if (!get().hydratedSets[code]) {
          // Mark hydrated up-front so concurrent callers don't double-fetch;
          // we'd rather fail-soft than spam Scryfall.
          set((state) => {
            state.hydratedSets[code] = true;
          });
          const cards = await fetchCardsBySet(code);
          set((state) => {
            for (const card of cards) {
              const uris = chooseImageUrisForCard(card, { frontOnly: true });
              if (!uris) continue;
              const entry: CardEntry = { info: card, texture: Texture.EMPTY, uris };
              const lowerName = card.name.toLowerCase();
              const setLower = card.set.toLowerCase();
              const cn = card.collector_number.toLowerCase();
              state.cards[`set:${setLower}::cn:${cn}`] = { card: entry };
              state.cards[`name:${lowerName}`] = { card: entry };
              state.cards[`name:${lowerName}::set:${setLower}`] = { card: entry };
              state.cards[`id:${card.id}`] = { card: entry };
            }
          });
        }
        return Object.values(get().cards)
          .map((e) => e.card?.info)
          .filter((c): c is ScryfallCard => !!c && c.set?.toLowerCase() === code);
      },
      prefetchSet: async (setCode) => {
        const cards = await get().hydrateSet(setCode);
        if (cards.length === 0) return;
        // Warm the browser HTTP cache for every card image — `<img>`
        // tags in the deck-builder will then resolve instantly. We
        // hit `normal` because that's what `CardThumbnail` renders;
        // PIXI textures (`getCardTexture`) are reserved for the game
        // canvas and would over-fetch here.
        if (typeof Image === "undefined") return;
        for (const c of cards) {
          const uris = chooseImageUrisForCard(c, { frontOnly: true });
          if (!uris?.normal) continue;
          const img = new Image();
          img.src = uris.normal;
        }
      },
      updatePrinting: (print) => {
        const lowerName = print.name.toLowerCase();
        const setCode = print.set.toLowerCase();
        const collectorNumber = print.collector_number.toLowerCase();
        const setCnKey = `set:${setCode}::cn:${collectorNumber}`;
        const uris = chooseImageUrisForCard(print, { frontOnly: true });
        if (!uris) {
          throw new Error("Couldnt find uris for printing: " + setCnKey);
        }
        set((state) => {
          // Invalidate every cache entry tied to this card name so stale
          // prints (especially under name-only keys) don't shadow the new one.
          for (const k of Object.keys(state.cards)) {
            const cachedName = state.cards[k].card?.info.name?.toLowerCase();
            if (
              cachedName === lowerName ||
              k === `name:${lowerName}` ||
              k.startsWith(`name:${lowerName}::`)
            ) {
              delete state.cards[k];
            }
          }
          const entry: CardEntry = { info: print, texture: Texture.EMPTY, uris };
          state.cards[setCnKey] = { card: entry };
          state.cards[`name:${lowerName}`] = { card: entry };
          state.cards[`name:${lowerName}::set:${setCode}`] = { card: entry };
          state.cards[`id:${print.id}`] = { card: entry };
        });
        return get().cards[setCnKey].card!;
      },
      invalidateCard: (name) => {
        const lowerName = name.toLowerCase();
        set((state) => {
          for (const k of Object.keys(state.cards)) {
            const cachedName = state.cards[k].card?.info.name?.toLowerCase();
            if (
              cachedName === lowerName ||
              k === `name:${lowerName}` ||
              k.startsWith(`name:${lowerName}::`)
            ) {
              delete state.cards[k];
            }
          }
        });
        pendingTexturePromises.forEach((_, k) => {
          if (k === `name:${lowerName}` || k.startsWith(`name:${lowerName}::`)) {
            pendingTexturePromises.delete(k);
          }
        });
      },
      init: async () => {
        const sets = await fetchSets();
        set((state) => {
          state.sets = sets;
        });
      },
    })),
    { name: "scryfall", enabled: import.meta.env.DEV },
  ),
);

export const useCard = (
  c: { name: string; setCode?: string; cardNumber?: string } | null | undefined,
) => {
  const getCard = useScryfallStore((s) => s.getCard);
  const name = c?.name;
  const setCode = c?.setCode;
  const cardNumber = c?.cardNumber;
  // Some prompts have no source card (e.g. keyword-driven dice modifiers).
  // Treat that as a no-op rather than throwing inside `scryfallLookupKey`.
  const hasLookup = Boolean(name) || Boolean(setCode && cardNumber);
  const key = hasLookup ? scryfallLookupKey({ name, setCode, collectorNumber: cardNumber }) : null;
  const cached = useScryfallStore((s) => (key ? (s.cards[key]?.card ?? null) : null));

  useEffect(() => {
    if (!hasLookup || cached) return;
    void getCard({ name, setCode, collectorNumber: cardNumber });
  }, [getCard, name, setCode, cardNumber, cached, key, hasLookup]);
  return cached;
};
export const useCardTexture = (
  c: { name: string; setCode?: string; cardNumber?: string } | null | undefined,
) => {
  const getCardTexture = useScryfallStore((s) => s.getCardTexture);
  const name = c?.name;
  const setCode = c?.setCode;
  const cardNumber = c?.cardNumber;
  const [out, setOut] = useState<CardEntry | null>(null);

  useEffect(() => {
    if (!name) return;
    let cancelled = false;
    getCardTexture({ name, setCode, collectorNumber: cardNumber }).then((v) => {
      if (!cancelled) setOut(v);
    });
    return () => {
      cancelled = true;
    };
  }, [getCardTexture, name, setCode, cardNumber]);
  return out;
};

export const useCardRulings = (card: { rulings_uri: string }) => {
  const getRulings = useScryfallStore((s) => s.getRulings);
  const [out, setOut] = useState<ScryfallRulingsResponse | null>(null);
  useEffect(() => {
    getRulings(card).then(setOut);
  }, [getRulings, card]);
  return out;
};

export interface PrefetchProgress {
  loaded: number;
  failed: number;
  total: number;
}

/**
 * Eagerly fetch every Scryfall texture for the given card identities and
 * resolve only once every request has settled.
 *
 * Used to gate the game-start handoff so the engine doesn't begin
 * emitting prompts (which would dismiss the loading screen) before
 * card artwork is in the texture cache. Failures don't reject the
 * outer promise — the engine is still allowed to start, missing-art
 * cards just fall back to the text rendering — but the failed count
 * surfaces in `onProgress` so the UI can flag it.
 */
export async function prefetchCards(
  cards: { name: string; setCode?: string; cardNumber?: string }[],
  onProgress?: (progress: PrefetchProgress) => void,
): Promise<PrefetchProgress> {
  const state = useScryfallStore.getState();
  const total = cards.length;
  let loaded = 0;
  let failed = 0;
  onProgress?.({ loaded, failed, total });
  await Promise.all(
    cards.map(async (c) => {
      try {
        await state.getCardTexture({
          name: c.name,
          setCode: c.setCode,
          collectorNumber: c.cardNumber,
        });
        loaded += 1;
      } catch (err) {
        failed += 1;
        console.warn(`[scryfall] prefetch failed for ${c.name}:`, err);
      }
      onProgress?.({ loaded, failed, total });
    }),
  );
  return { loaded, failed, total };
}

export function useSetLookup(): Map<string, ScryfallSet> {
  const sets = useScryfallStore((s) => s.sets);
  if (!sets) return new Map();
  return new Map(sets.map((s) => [s.code, s]));
}
