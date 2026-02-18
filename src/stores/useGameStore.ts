import { create } from 'zustand';
import type { GameView } from '@/types/xmage';

interface GameState {
  gameView: GameView | null;
  updateGameView: (view: GameView) => void;
  // Actions
  castSpell: (cardId: string) => void;
  passPriority: () => void;
}

export const useGameStore = create<GameState>((set) => ({
  gameView: null,
  updateGameView: (view) => set({ gameView: view }),
  castSpell: (cardId) => {
    console.log('Casting spell:', cardId);
    // Send to middleware
  },
  passPriority: () => {
    console.log('Passing priority');
    // Send to middleware
  },
}));
