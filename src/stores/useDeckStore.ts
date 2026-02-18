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
    set((state) => ({
      currentDeck: { ...state.currentDeck, cards: state.currentDeck.cards.filter((c) => c.id !== cardId) },
    })),
  removeFromSide: (cardId) =>
    set((state) => ({
      currentDeck: { ...state.currentDeck, sideboard: state.currentDeck.sideboard.filter((c) => c.id !== cardId) },
    })),
  clearDeck: () => set({ currentDeck: initialDeck }),
  loadDeck: (deck) => set({ currentDeck: deck }),
}));
