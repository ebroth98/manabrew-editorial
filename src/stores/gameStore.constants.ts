import type { AgentPrompt, GameState, DeferredSnapshot } from './gameStore.types';
import type { GameLogEntry } from '@/types/gameLog';

/** Prompt types the UI knows how to render a modal/interaction for. */
export const HANDLED_PROMPT_TYPES = new Set([
  "stateUpdate",
  "gameOver",
  "mulligan",
  "mulliganPutBack",
  "chooseAction",
  "chooseAttackers",
  "chooseBlockers",
  "chooseTargetCard",
  "chooseTargetCardFromZone",
  "chooseTargetPlayer",
  "chooseTargetAny",
  "chooseTargetSpell",
  "chooseMode",
  "chooseOptionalTrigger",
  "choosePhyrexian",
  "chooseKicker",
  "chooseBuyback",
  "chooseMultikicker",
  "chooseReplicate",
  "chooseAlternativeCost",
  "chooseColor",
  "chooseCardsForEffect",
  "chooseType",
  "chooseNumber",
  "chooseCardName",
  "chooseDiscard",
  "chooseDamageAssignmentOrder",
  "payCombatCost",
  "payManaCost",
  "chooseDelve",
  "chooseConvoke",
  "chooseImprovise",
  "specifyManaCombo",
  "scry",
  "surveil",
  "dig",
  "chooseExertAttackers",
  "chooseEnlistAttackers",
  "reorderLibrary",
  "exploreDecision",
  "helpPayAssist",
]);

export function applyPrompt(prompt: AgentPrompt, source: string, set: (partial: Partial<GameState>) => void, get: () => GameState) {
  const displayEvents = [...(prompt.displayEvents ?? [])];
  // Don't mutate the original payload (listeners may fire more than once).

  const currentGameView = get().gameView;
  const queueLen = get().deferredQueue.length;
  // stateUpdate prompts only carry a gameView + display events — they should
  // NOT replace the currentPrompt (the active player decision).
  const isStateUpdate = prompt.type === "stateUpdate";

  // DEV warning: detect prompt types the UI doesn't handle (engine takes a default/arbitrary action)
  if (!isStateUpdate && !HANDLED_PROMPT_TYPES.has(prompt.type)) {
    const cardName = prompt.sourceCardName ?? prompt.cardName ?? prompt.attackerName ?? "unknown";
    const details = JSON.stringify(prompt, null, 2);
    const devMsg = `[DEV] Unhandled prompt "${prompt.type}" for card "${cardName}" — engine takes default action\n${details}`;
    console.warn(devMsg, prompt);
    const devEntry: GameLogEntry = {
      message: devMsg,
      entryType: "warning",
      timestampMs: Date.now(),
    };
    set({ gameLog: [...get().gameLog.slice(-99), devEntry] });
  }

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
      currentPrompt: isStateUpdate ? null : prompt,
    };
    set(updates);
  }
}
