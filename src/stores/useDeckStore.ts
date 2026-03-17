import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Deck, Card } from '@/types/xmage';

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
  /** Patch cards in a specific saved deck by name with enriched data from Scryfall. */
  enrichSavedDeck: (id: string, updates: Map<string, Partial<Card>>) => void;
}

const initialDeck: Deck = {
  name: 'New Deck',
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
          const updates = new Map();
          updates.set(cardName.toLowerCase(), {
            setCode: scryfallCard.set,
            imageUrl: scryfallCard.image_uris?.normal ?? scryfallCard.image_uris?.large ?? null,
            cardNumber: scryfallCard.collector_number,
          });
          function applyUpdates(cards: Card[]): Card[] {
            return cards.map((c) => {
              const patch = updates.get(c.name.toLowerCase());
              return patch ? { ...c, ...patch } : c;
            });
          }
          const cmd = state.currentDeck.commander;
          const cmdPatch = cmd ? updates.get(cmd.name.toLowerCase()) : undefined;
          return {
            currentDeck: {
              ...state.currentDeck,
              cards: applyUpdates(state.currentDeck.cards),
              sideboard: applyUpdates(state.currentDeck.sideboard),
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
          function applyUpdates(cards: Card[]): Card[] {
            return cards.map((c) => {
              const patch = updates.get(c.name.toLowerCase());
              return patch ? { ...c, ...patch } : c;
            });
          }
          const cmd = state.currentDeck.commander;
          const cmdPatch = cmd ? updates.get(cmd.name.toLowerCase()) : undefined;
          return {
            currentDeck: {
              ...state.currentDeck,
              cards: applyUpdates(state.currentDeck.cards),
              sideboard: applyUpdates(state.currentDeck.sideboard),
              ...(cmdPatch ? { commander: { ...cmd!, ...cmdPatch } } : {}),
            },
          };
        }),
      enrichSavedDeck: (id, updates) =>
        set((state) => {
          function applyUpdates(cards: Card[]): Card[] {
            return cards.map((c) => {
              const patch = updates.get(c.name.toLowerCase());
              return patch ? { ...c, ...patch } : c;
            });
          }
          return {
            savedDecks: state.savedDecks.map((s) =>
              s.id !== id
                ? s
                : {
                    ...s,
                    deck: {
                      ...s.deck,
                      cards: applyUpdates(s.deck.cards),
                      sideboard: applyUpdates(s.deck.sideboard),
                    },
                  }
            ),
          };
        }),
    }),
    {
      name: 'xmage-deck-storage',
    }
  )
);
