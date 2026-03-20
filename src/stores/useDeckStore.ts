import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Deck, Card } from '@/types/openmagic';
import { STORAGE_KEYS, DEFAULT_DECK_NAME } from '@/lib/constants';

/** Apply a map of name→patch to an array of cards. */
function patchCardsByName(cards: Card[], updates: Map<string, Partial<Card>>): Card[] {
  return cards.map((c) => {
    const patch = updates.get(c.name.toLowerCase());
    return patch ? { ...c, ...patch } : c;
  });
}

export interface SavedDeck {
  id: string;
  deck: Deck;
  savedAt: number;
}

interface DeckState {
  currentDeck: Deck;
  pool: Card[];
  savedDecks: SavedDeck[];
  addToMain: (card: Card) => void;
  addToSide: (card: Card) => void;
  addToPool: (card: Card) => void;
  removeFromMain: (cardId: string) => void;
  removeFromSide: (cardId: string) => void;
  setDeckName: (name: string) => void;
  clearDeck: () => void;
  loadDeck: (deck: Deck) => void;
  saveCurrentDeck: () => void;
  loadSavedDeck: (id: string) => void;
  deleteSavedDeck: (id: string) => void;
  setCommander: (card: Card) => void;
  removeCommander: () => void;
  updatePrint: (cardName: string, scryfallCard: import('@/types/scryfall').ScryfallCard) => void;
  /** Patch cards in currentDeck by name with enriched data from Scryfall. */
  enrichDeckCards: (updates: Map<string, Partial<Card>>) => void;
  /** Add a card to a saved deck's main board. */
  addCardToSavedDeck: (id: string, card: Card) => void;
  /** Patch cards in a specific saved deck by name with enriched data from Scryfall. */
  enrichSavedDeck: (id: string, updates: Map<string, Partial<Card>>) => void;
  /** Add a custom tag/section to the current deck. */
  addCustomTag: (tag: string) => void;
  /** Remove a custom tag/section and all card associations. */
  removeCustomTag: (tag: string) => void;
  /** Assign a tag to a card (by name). */
  tagCard: (cardName: string, tag: string) => void;
  /** Remove a tag from a card (by name). */
  untagCard: (cardName: string, tag: string) => void;
  /** Add a label to the current deck. */
  addDeckLabel: (label: string, color?: string) => void;
  /** Remove a label from the current deck. */
  removeDeckLabel: (label: string) => void;
  /** Update the color of an existing label. */
  updateDeckLabelColor: (label: string, color?: string) => void;
}

const initialDeck: Deck = {
  name: DEFAULT_DECK_NAME,
  cards: [],
  sideboard: [],
};

export const useDeckStore = create<DeckState>()(
  persist(
    (set) => ({
      currentDeck: initialDeck,
      pool: [],
      savedDecks: [],
      addToMain: (card) =>
        set((state) => ({
          currentDeck: { ...state.currentDeck, cards: [...state.currentDeck.cards, card] },
        })),
      addToSide: (card) =>
        set((state) => ({
          currentDeck: { ...state.currentDeck, sideboard: [...state.currentDeck.sideboard, card] },
        })),
      addToPool: (card) =>
        set((state) => ({
          pool: [...state.pool, card],
        })),
      removeFromMain: (cardId) =>
        set((state) => {
          const index = state.currentDeck.cards.findIndex((c) => c.id === cardId);
          if (index === -1) return state;
          const newCards = [...state.currentDeck.cards];
          newCards.splice(index, 1);
          return { currentDeck: { ...state.currentDeck, cards: newCards } };
        }),
      removeFromSide: (cardId) =>
        set((state) => {
          const index = state.currentDeck.sideboard.findIndex((c) => c.id === cardId);
          if (index === -1) return state;
          const newSide = [...state.currentDeck.sideboard];
          newSide.splice(index, 1);
          return { currentDeck: { ...state.currentDeck, sideboard: newSide } };
        }),
      setDeckName: (name) =>
        set((state) => ({
          currentDeck: { ...state.currentDeck, name },
        })),
      clearDeck: () => set({ currentDeck: { ...initialDeck } }),
      loadDeck: (deck) => set({ currentDeck: deck }),
      setCommander: (card) =>
        set((state) => ({
          currentDeck: { ...state.currentDeck, commander: card },
        })),
      removeCommander: () =>
        set((state) => ({
          currentDeck: { ...state.currentDeck, commander: undefined },
        })),
      updatePrint: (cardName, scryfallCard) =>
        set((state) => {
          const updates = new Map<string, Partial<Card>>();
          updates.set(cardName.toLowerCase(), {
            setCode: scryfallCard.set,
            imageUrl: scryfallCard.image_uris?.normal ?? scryfallCard.image_uris?.large ?? null,
            cardNumber: scryfallCard.collector_number,
          });
          const cmd = state.currentDeck.commander;
          const cmdPatch = cmd ? updates.get(cmd.name.toLowerCase()) : undefined;
          return {
            currentDeck: {
              ...state.currentDeck,
              cards: patchCardsByName(state.currentDeck.cards, updates),
              sideboard: patchCardsByName(state.currentDeck.sideboard, updates),
              ...(cmdPatch ? { commander: { ...cmd!, ...cmdPatch } } : {}),
            },
          };
        }),
      saveCurrentDeck: () =>
        set((state) => {
          const existing = state.savedDecks.find((s) => s.deck.name === state.currentDeck.name);
          if (existing) {
            return {
              savedDecks: state.savedDecks.map((s) =>
                s.id === existing.id ? { ...s, deck: state.currentDeck, savedAt: Date.now() } : s
              ),
            };
          }
          const newSaved: SavedDeck = {
            id: crypto.randomUUID(),
            deck: state.currentDeck,
            savedAt: Date.now(),
          };
          return { savedDecks: [...state.savedDecks, newSaved] };
        }),
      loadSavedDeck: (id) =>
        set((state) => {
          const found = state.savedDecks.find((s) => s.id === id);
          if (!found) return state;
          return { currentDeck: found.deck };
        }),
      deleteSavedDeck: (id) =>
        set((state) => ({
          savedDecks: state.savedDecks.filter((s) => s.id !== id),
        })),
      enrichDeckCards: (updates) =>
        set((state) => {
          const cmd = state.currentDeck.commander;
          const cmdPatch = cmd ? updates.get(cmd.name.toLowerCase()) : undefined;
          return {
            currentDeck: {
              ...state.currentDeck,
              cards: patchCardsByName(state.currentDeck.cards, updates),
              sideboard: patchCardsByName(state.currentDeck.sideboard, updates),
              ...(cmdPatch ? { commander: { ...cmd!, ...cmdPatch } } : {}),
            },
          };
        }),
      addCardToSavedDeck: (id, card) =>
        set((state) => ({
          savedDecks: state.savedDecks.map((s) =>
            s.id !== id
              ? s
              : { ...s, deck: { ...s.deck, cards: [...s.deck.cards, card] }, savedAt: Date.now() },
          ),
        })),
      enrichSavedDeck: (id, updates) =>
        set((state) => ({
          savedDecks: state.savedDecks.map((s) =>
            s.id !== id
              ? s
              : {
                  ...s,
                  deck: {
                    ...s.deck,
                    cards: patchCardsByName(s.deck.cards, updates),
                    sideboard: patchCardsByName(s.deck.sideboard, updates),
                  },
                }
          ),
        })),
      addCustomTag: (tag) =>
        set((state) => {
          const existing = state.currentDeck.customTags ?? [];
          if (existing.includes(tag)) return state;
          return {
            currentDeck: { ...state.currentDeck, customTags: [...existing, tag] },
          };
        }),
      removeCustomTag: (tag) =>
        set((state) => {
          const customTags = (state.currentDeck.customTags ?? []).filter((t) => t !== tag);
          const cardTags = { ...state.currentDeck.cardTags };
          for (const key of Object.keys(cardTags)) {
            cardTags[key] = cardTags[key].filter((t) => t !== tag);
            if (cardTags[key].length === 0) delete cardTags[key];
          }
          return {
            currentDeck: { ...state.currentDeck, customTags, cardTags },
          };
        }),
      tagCard: (cardName, tag) =>
        set((state) => {
          const key = cardName.toLowerCase();
          const cardTags = { ...state.currentDeck.cardTags };
          const tags = cardTags[key] ?? [];
          if (tags.includes(tag)) return state;
          cardTags[key] = [...tags, tag];
          return {
            currentDeck: { ...state.currentDeck, cardTags },
          };
        }),
      untagCard: (cardName, tag) =>
        set((state) => {
          const key = cardName.toLowerCase();
          const cardTags = { ...state.currentDeck.cardTags };
          const tags = cardTags[key] ?? [];
          cardTags[key] = tags.filter((t) => t !== tag);
          if (cardTags[key].length === 0) delete cardTags[key];
          return {
            currentDeck: { ...state.currentDeck, cardTags },
          };
        }),
      addDeckLabel: (label, color) =>
        set((state) => {
          const existing = state.currentDeck.labels ?? [];
          if (existing.some((l) => l.name.toLowerCase() === label.toLowerCase())) return state;
          return {
            currentDeck: { ...state.currentDeck, labels: [...existing, { name: label, color }] },
          };
        }),
      removeDeckLabel: (label) =>
        set((state) => ({
          currentDeck: {
            ...state.currentDeck,
            labels: (state.currentDeck.labels ?? []).filter((l) => l.name !== label),
          },
        })),
      updateDeckLabelColor: (label, color) =>
        set((state) => ({
          currentDeck: {
            ...state.currentDeck,
            labels: (state.currentDeck.labels ?? []).map((l) =>
              l.name === label ? { ...l, color } : l
            ),
          },
        })),
    }),
    {
      name: STORAGE_KEYS.DECK,
    }
  )
);
