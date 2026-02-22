import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { GameView, Card } from '@/types/xmage';
import { getFormat } from '@/lib/formats';

interface DisplayEvent {
  kind: string;
  cardId?: string;
  cardName?: string;
  playerId?: string;
  activePlayerId?: string;
  activePlayerName?: string;
  turnNumber?: number;
}

interface AgentPrompt {
  type: string;
  gameView: GameView;
  displayEvents?: DisplayEvent[];
  playableCardIds?: string[];
  handCardIds?: string[];
  availableAttackerIds?: string[];
  attackerIds?: string[];
  availableBlockerIds?: string[];
  validPlayerIds?: string[];
  validCardIds?: string[];
  tappableLandIds?: string[];
  untappableLandIds?: string[];
  zone?: string;
  zoneCards?: Card[];
  /** IDs of library cards revealed for scry / surveil / dig */
  cardIds?: string[];
  /** Card DTOs for the revealed library cards */
  cards?: Card[];
  /** dig: maximum number of cards the player may take */
  numToTake?: number;
  /** dig: whether taking 0 cards is allowed */
  optional?: boolean;
  /** chooseDiscard: how many must be discarded */
  numToDiscard?: number;
  /** chooseTargetSpell: stack entry IDs that can be countered */
  validSpellIds?: string[];
  /** chooseMode: human-readable descriptions for each available mode */
  options?: string[];
  /** chooseMode: minimum number of modes that must be chosen */
  minChoices?: number;
  /** chooseMode: maximum number of modes that can be chosen */
  maxChoices?: number;
}

interface GameConfig {
  formatId: string;
  startingLife: number;
}

/** A snapshot queued for sequential flash-then-apply processing. */
interface DeferredSnapshot {
  displayEvents: DisplayEvent[];
  gameView: GameView;
  /** null for display-only state updates (no player decision). */
  prompt: AgentPrompt | null;
}

interface GameState {
  gameView: GameView | null;
  currentPrompt: AgentPrompt | null;
  gameLog: string[];
  isGameActive: boolean;
  debugInfo: string;
  /** Queue of deferred snapshots waiting for flash animation. */
  deferredQueue: DeferredSnapshot[];
  /** True while Game.tsx is processing flash animations. */
  isFlashing: boolean;
  /** True after respond() is called and before the next prompt arrives — prevents double-submit. */
  isWaitingForResponse: boolean;
  gameConfig: GameConfig | null;
  updateGameView: (view: GameView) => void;
  setGameConfig: (config: GameConfig) => void;
  // Actions
  startGame: (cardNames: string[], formatId?: string, commanderName?: string) => Promise<void>;
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
  scryDecision: (bottomCardIds: string[]) => void;
  surveilDecision: (graveyardCardIds: string[]) => void;
  digDecision: (chosenCardIds: string[]) => void;
  discardDecision: (discardedCardIds: string[]) => void;
  targetSpell: (spellId: string | null) => void;
  modeDecision: (chosenIndices: number[]) => void;
  concede: () => void;
  endGame: () => Promise<void>;
  setupListeners: () => Promise<() => void>;
}

function applyPrompt(prompt: AgentPrompt, source: string, set: (partial: Partial<GameState>) => void, get: () => GameState) {
  const displayEvents = [...(prompt.displayEvents ?? [])];
  // Don't mutate the original payload (listeners may fire more than once).

  const currentGameView = get().gameView;
  const queueLen = get().deferredQueue.length;
  // stateUpdate prompts only carry a gameView + display events — they should
  // NOT replace the currentPrompt (the active player decision).
  const isStateUpdate = prompt.type === "stateUpdate";

  if (displayEvents.length > 0 && currentGameView !== null) {
    // Enqueue this snapshot — the flash processor will play the events then apply the state.
    const snapshot: DeferredSnapshot = { displayEvents, gameView: prompt.gameView, prompt: isStateUpdate ? null : prompt };
    set({
      deferredQueue: [...get().deferredQueue, snapshot],
      debugInfo: `${source}: ${prompt.type} (queued #${queueLen + 1})`,
    });
  } else if (queueLen > 0 || get().isFlashing) {
    // Flashes are in progress but this prompt has no display events — enqueue with empty events
    // so it gets applied after the current flash sequence finishes.
    const snapshot: DeferredSnapshot = { displayEvents: [], gameView: prompt.gameView, prompt: isStateUpdate ? null : prompt };
    set({
      deferredQueue: [...get().deferredQueue, snapshot],
      debugInfo: `${source}: ${prompt.type} (queued-passthrough #${queueLen + 1})`,
    });
  } else {
    // No display events and no queue — apply immediately
    const updates: Partial<GameState> = {
      gameView: prompt.gameView,
      debugInfo: `${source}: ${prompt.type}`,
      isWaitingForResponse: false,
    };
    if (!isStateUpdate) {
      updates.currentPrompt = prompt;
    }
    set(updates);
  }
}

export const useGameStore = create<GameState>((set, get) => ({
  gameView: null,
  currentPrompt: null,
  gameLog: [],
  isGameActive: false,
  debugInfo: '',
  deferredQueue: [],
  isFlashing: false,
  isWaitingForResponse: false,
  gameConfig: null,

  updateGameView: (view) => set({ gameView: view }),

  setGameConfig: (config) => set({ gameConfig: config }),

  startGame: async (cardNames, formatId, commanderName) => {
    try {
      set({ debugInfo: 'Starting game...' });
      const format = formatId ? getFormat(formatId) : undefined;
      const startingLife = format?.deckRules.startingLife ?? 20;
      const gameConfig: GameConfig = { formatId: formatId ?? 'constructed', startingLife };
      set({ gameConfig });
      const result = await invoke('start_game', {
        deckList: cardNames,
        startingLife,
        commanderName: commanderName ?? null,
      });
      // Clear old game state so stale gameView/prompts don't bleed into new game
      set({ isGameActive: true, gameLog: [], gameView: null, currentPrompt: null, deferredQueue: [], isFlashing: false, isWaitingForResponse: false, debugInfo: `Game started: ${result}. Polling...` });
    } catch (e) {
      set({ debugInfo: `Start failed: ${e}` });
      console.error('[store] Failed to start game:', e);
    }
  },

  respond: async (action) => {
    try {
      // Mark as waiting so action buttons are disabled while the engine processes the response.
      // We do NOT clear currentPrompt here — keeping it visible prevents a blank-state flash.
      // The incoming next prompt will clear isWaitingForResponse and update currentPrompt.
      set({ isWaitingForResponse: true, debugInfo: `Responding: ${action.type}` });
      await invoke('respond', { action });
    } catch (e) {
      set({ isWaitingForResponse: false, debugInfo: `Respond error: ${e}` });
      console.error('Failed to respond:', e);
    }
  },

  castSpell: (cardId) => {
    get().respond({ type: 'playCard', cardId });
  },

  passPriority: () => {
    if (get().isWaitingForResponse) return;
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

  scryDecision: (bottomCardIds) => {
    get().respond({ type: 'scryDecision', bottomCardIds });
  },

  surveilDecision: (graveyardCardIds) => {
    get().respond({ type: 'surveilDecision', graveyardCardIds });
  },

  digDecision: (chosenCardIds) => {
    get().respond({ type: 'digDecision', chosenCardIds });
  },

  discardDecision: (discardedCardIds) => {
    get().respond({ type: 'discardDecision', discardedCardIds });
  },

  targetSpell: (spellId) => {
    get().respond({ type: 'targetSpell', spellId });
  },

  modeDecision: (chosenIndices) => {
    get().respond({ type: 'modeDecision', chosenIndices });
  },

  concede: () => {
    get().respond({ type: 'concede' });
  },

  endGame: async () => {
    try {
      await invoke('end_game');
      set({ isGameActive: false, gameView: null, currentPrompt: null, deferredQueue: [], isFlashing: false, isWaitingForResponse: false });
    } catch (e) {
      console.error('Failed to end game:', e);
    }
  },

  setupListeners: async () => {
    const unlisteners: UnlistenFn[] = [];

    try {
      const unlisten1 = await listen<AgentPrompt>('game:prompt', (event) => {
        const prompt = event.payload;
        if (get().gameView?.gameOver) return;
        if (prompt && prompt.gameView) {
          applyPrompt(prompt, 'Event', set, get);
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
