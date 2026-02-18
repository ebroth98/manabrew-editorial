import { create } from 'zustand';
import type { Deck, Card } from '@/types/xmage';

interface DeckState {
  currentDeck: Deck;
  pool: Card[];
  addToMain: (card: Card) => void;
  addToSide: (card: Card) => void;
  addToPool: (card: Card) => void;
  removeFromMain: (cardId: string) => void;
  removeFromSide: (cardId: string) => void;
  clearDeck: () => void;
  loadDeck: (deck: Deck) => void;
}

const initialDeck: Deck = {
  name: 'New Deck',
  cards: [],
  sideboard: [],
};

export const useDeckStore = create<DeckState>((set) => ({
  currentDeck: initialDeck,
  pool: [],
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
  clearDeck: () => set({ currentDeck: initialDeck }),
  loadDeck: (deck) => set({ currentDeck: deck }),
}));
