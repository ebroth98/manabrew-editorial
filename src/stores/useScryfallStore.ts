import { create } from "zustand";
import { devtools } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";
import {
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
  getCard: (lookup: ScryfallCardLookup) => Promise<CardEntry>;
  getCardTexture: (lookup: ScryfallCardLookup) => Promise<CardEntry>;
  updatePrinting: (card: ScryfallCard) => CardEntry;
  invalidateCard: (name: string) => void;
  getRulings: (card: { rulings_uri: string }) => Promise<ScryfallRulingsResponse>;
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

export const useScryfallStore = create<ScryfallState>()(
  devtools(
    immer((set, get) => ({
      cards: {},
      _fetchCardLookup: async (lookup) => {
        const key = scryfallLookupKey(lookup);
        console.log("===== ACTUALLY FETCHING: " + key);
        const card = await fetchScryfallCard(lookup);

        const uris = chooseImageUrisForCard(card, { frontOnly: true });
        if (!uris) {
          throw new Error("Couldn't find a texture url for: " + JSON.stringify(lookup));
        }

        const entry = {
          card: { info: card, texture: Texture.EMPTY, uris },
        };
        set((state) => {
          state.cards[key] = entry;
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
          set((state) => {
            state.cards[key] = { card: entry };
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
  const key = name ? scryfallLookupKey({ name, setCode, collectorNumber: cardNumber }) : null;
  const cached = useScryfallStore((s) => (key ? (s.cards[key]?.card ?? null) : null));

  useEffect(() => {
    if (!name || cached) return;
    void getCard({ name, setCode, collectorNumber: cardNumber });
  }, [getCard, name, setCode, cardNumber, cached, key]);
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

export async function prefetchCards(
  cards: { name: string; setCode?: string; cardNumber?: string }[],
): Promise<void> {
  const state = useScryfallStore.getState();
  await Promise.allSettled(
    cards.map((c) =>
      state.getCardTexture({ name: c.name, setCode: c.setCode, collectorNumber: c.cardNumber }),
    ),
  );
}

export function useSetLookup(): Map<string, ScryfallSet> {
  const sets = useScryfallStore((s) => s.sets);
  if (!sets) return new Map();
  return new Map(sets.map((s) => [s.code, s]));
}
