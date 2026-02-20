import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { GameView } from '@/types/xmage';

interface AgentPrompt {
  type: string;
  gameView: GameView;
  playableCardIds?: string[];
  handCardIds?: string[];
  availableAttackerIds?: string[];
  attackerIds?: string[];
  availableBlockerIds?: string[];
  validPlayerIds?: string[];
  validCardIds?: string[];
  tappableLandIds?: string[];
  untappableLandIds?: string[];
}

interface GameState {
  gameView: GameView | null;
  currentPrompt: AgentPrompt | null;
  gameLog: string[];
  isGameActive: boolean;
  debugInfo: string;
  updateGameView: (view: GameView) => void;
  // Actions
  startGame: (deckChoice: string) => Promise<void>;
  respond: (action: Record<string, unknown>) => Promise<void>;
  castSpell: (cardId: string) => void;
  passPriority: () => void;
  declareAttackers: (attackerIds: string[]) => void;
  declareBlockers: (assignments: { blockerId: string; attackerId: string }[]) => void;
  targetPlayer: (playerId: string | null) => void;
  targetCard: (cardId: string | null) => void;
  targetAny: (target: { kind: string; playerId?: string; cardId?: string }) => void;
  mulliganDecision: (keep: boolean) => void;
  tapLand: (cardId: string) => void;
  untapLand: (cardId: string) => void;
  concede: () => void;
  endGame: () => Promise<void>;
  setupListeners: () => Promise<() => void>;
  pollForPrompt: () => Promise<void>;
}

export const useGameStore = create<GameState>((set, get) => ({
  gameView: null,
  currentPrompt: null,
  gameLog: [],
  isGameActive: false,
  debugInfo: '',

  updateGameView: (view) => set({ gameView: view }),

  startGame: async (deckChoice) => {
    try {
      set({ debugInfo: 'Starting game...' });
      const result = await invoke('start_game', { deckChoice });
      // Clear old game state so stale gameView/prompts don't bleed into new game
      set({ isGameActive: true, gameLog: [], gameView: null, currentPrompt: null, debugInfo: `Game started: ${result}. Polling...` });
      // Poll for the first prompt after a short delay
      setTimeout(() => get().pollForPrompt(), 500);
    } catch (e) {
      set({ debugInfo: `Start failed: ${e}` });
      console.error('[store] Failed to start game:', e);
    }
  },

  pollForPrompt: async () => {
    // Don't overwrite a game-over state with subsequent prompts
    if (get().gameView?.gameOver) return;
    try {
      const prompt = await invoke<AgentPrompt | null>('get_prompt');
      if (prompt && prompt.gameView) {
        set({
          gameView: prompt.gameView,
          currentPrompt: prompt,
          debugInfo: `Got prompt via poll: ${prompt.type}`,
        });
      } else {
        set({ debugInfo: `Poll returned: ${JSON.stringify(prompt)?.slice(0, 100)}` });
        // Retry after a delay
        setTimeout(() => {
          if (get().isGameActive && !get().gameView) {
            get().pollForPrompt();
          }
        }, 500);
      }
    } catch (e) {
      set({ debugInfo: `Poll error: ${e}` });
    }
  },

  respond: async (action) => {
    try {
      set({ debugInfo: `Responding: ${action.type}` });
      await invoke('respond', { action });
      // Poll for next prompt after responding
      setTimeout(() => get().pollForPrompt(), 200);
    } catch (e) {
      set({ debugInfo: `Respond error: ${e}` });
      console.error('Failed to respond:', e);
    }
  },

  castSpell: (cardId) => {
    get().respond({ type: 'playCard', cardId });
  },

  passPriority: () => {
    const prompt = get().currentPrompt;
    if (!prompt) return;
    switch (prompt.type) {
      case 'chooseAction':
        get().respond({ type: 'playCard', cardId: null });
        break;
      case 'chooseAttackers':
        get().respond({ type: 'declareAttackers', attackerIds: [] });
        break;
      case 'chooseBlockers':
        get().respond({ type: 'declareBlockers', assignments: [] });
        break;
      default:
        get().respond({ type: 'playCard', cardId: null });
    }
  },

  declareAttackers: (attackerIds) => {
    get().respond({ type: 'declareAttackers', attackerIds });
  },

  declareBlockers: (assignments) => {
    get().respond({ type: 'declareBlockers', assignments });
  },

  targetPlayer: (playerId) => {
    get().respond({ type: 'targetPlayer', playerId });
  },

  targetCard: (cardId) => {
    get().respond({ type: 'targetCard', cardId });
  },

  targetAny: (target) => {
    get().respond({ type: 'targetAny', target });
  },

  mulliganDecision: (keep) => {
    get().respond({ type: 'mulliganDecision', keep });
  },

  tapLand: (cardId) => {
    get().respond({ type: 'tapLand', cardId });
  },

  untapLand: (cardId) => {
    get().respond({ type: 'untapLand', cardId });
  },

  concede: () => {
    // Send concede to the backend; the GameOver prompt will update state
    get().respond({ type: 'concede' });
  },

  endGame: async () => {
    try {
      await invoke('end_game');
      set({ isGameActive: false, gameView: null, currentPrompt: null });
    } catch (e) {
      console.error('Failed to end game:', e);
    }
  },

  setupListeners: async () => {
    const unlisteners: UnlistenFn[] = [];

    try {
      // Try both global and window-level listeners
      const unlisten1 = await listen<AgentPrompt>('game:prompt', (event) => {
        const prompt = event.payload;
        // Don't overwrite a game-over state with subsequent prompts from a dying thread
        if (get().gameView?.gameOver) return;
        if (prompt && prompt.gameView) {
          set({
            gameView: prompt.gameView,
            currentPrompt: prompt,
            debugInfo: `Event received: ${prompt.type}`,
          });
        }
      });
      unlisteners.push(unlisten1);

      const unlisten2 = await listen<string>('game:log', (event) => {
        set((state) => ({
          gameLog: [...state.gameLog.slice(-99), event.payload],
        }));
      });
      unlisteners.push(unlisten2);
    } catch (e) {
      console.error('[store] Failed to setup listeners:', e);
    }

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  },
}));
