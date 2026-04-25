/**
 * Drives the "put cards on the bottom of the library" picker that
 * replaces the old mulligan modal. The selection is consumed by both
 * `HandDisplayCool` (to render the red ring + drop) and the
 * `MulliganPutBack` prompt action (for the Confirm button's counter),
 * so keeping it behind a single hook prevents `Game.tsx` from
 * re-plumbing state, toggle, and dispatch every render.
 *
 * Resets automatically whenever the engine advances to a different
 * prompt so a cancelled mulligan doesn't leak picks into the next
 * decision.
 */
import { useCallback, useState } from "react";
import type { AgentPrompt } from "@/stores/useGameStore";
import { PromptType } from "@/types/promptType";

export interface MulliganSelection {
  /** True while the engine is asking the player to pick cards to
   *  send to the bottom of the library. */
  active: boolean;
  /** How many cards the engine expects the player to pick. */
  count: number;
  /** The player's current picks. */
  selected: Set<string>;
  /** Toggle a card in or out of the selection (no-op when full). */
  toggle: (cardId: string) => void;
  /** Fires the put-back decision; no-op if the selection count
   *  doesn't match what the engine asked for. */
  confirm: () => void;
}

export function useMulliganSelection(
  activePrompt: AgentPrompt | null,
  putBackDecision: (cardIds: string[]) => void,
): MulliganSelection {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const promptCount =
    activePrompt?.type === PromptType.MulliganPutBack ? (activePrompt.count ?? 0) : 0;

  const promptKey = `${activePrompt?.type ?? ""}:${activePrompt?.count ?? ""}`;
  const [prevPromptKey, setPrevPromptKey] = useState(promptKey);
  if (prevPromptKey !== promptKey) {
    setPrevPromptKey(promptKey);
    setSelected(new Set());
  }

  const toggle = useCallback(
    (cardId: string) => {
      setSelected((prev) => {
        const next = new Set(prev);
        if (next.has(cardId)) {
          next.delete(cardId);
        } else if (next.size < promptCount) {
          next.add(cardId);
        }
        return next;
      });
    },
    [promptCount],
  );

  const confirm = useCallback(() => {
    if (selected.size !== promptCount) return;
    putBackDecision([...selected]);
  }, [selected, promptCount, putBackDecision]);

  return {
    active: activePrompt?.type === PromptType.MulliganPutBack,
    count: promptCount,
    selected,
    toggle,
    confirm,
  };
}
