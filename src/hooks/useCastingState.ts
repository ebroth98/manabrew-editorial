import { useState, useEffect, useCallback, useMemo } from "react";
import type { Card as XMageCard } from "@/types/openmagic";
import type { AgentPrompt } from "@/stores/useGameStore";

/**
 * Prompt types that are part of the spell-casting flow.
 * While any of these are active, the casting card stays visible in the stack.
 */
const CASTING_PROMPT_TYPES = new Set([
  "chooseTargetCard",
  "chooseTargetPlayer",
  "chooseTargetAny",
  "chooseTargetCardFromZone",
  "chooseTargetSpell",
  "payManaCost",
  "payCombatCost",
  "chooseDelve",
  "chooseConvoke",
  "chooseImprovise",
  "specifyManaCombo",
]);

interface UseCastingStateOptions {
  currentPrompt: AgentPrompt | null | undefined;
  hand: XMageCard[];
  battlefield: XMageCard[];
  targetCard: (cardId: string | null) => void;
  targetPlayer: (playerId: string) => void;
  targetAny: (target: { kind: string; playerId?: string; cardId?: string }) => void;
}

export function useCastingState({
  currentPrompt,
  hand,
  battlefield,
  targetCard,
  targetPlayer,
  targetAny,
}: UseCastingStateOptions) {
  const promptType = currentPrompt?.type;

  // Derive the casting card ID from the prompt
  // sourceCardId is set by the engine on targeting prompts
  // cardId is set on PayManaCost and similar cost prompts
  const castingCardId = useMemo(() => {
    if (!promptType || !CASTING_PROMPT_TYPES.has(promptType)) return null;
    return currentPrompt?.sourceCardId ?? currentPrompt?.cardId ?? null;
  }, [promptType, currentPrompt?.sourceCardId, currentPrompt?.cardId]);

  // Resolve the actual card object — check hand first, then battlefield (activated abilities)
  const castingCard = useMemo(
    () => castingCardId
      ? hand.find((c) => c.id === castingCardId)
        ?? battlefield.find((c) => c.id === castingCardId)
        ?? null
      : null,
    [castingCardId, hand, battlefield],
  );

  // If the card is on the battlefield (activated ability), it should stay visible there
  const castingFromBattlefield = useMemo(
    () => castingCardId ? !hand.some((c) => c.id === castingCardId) && battlefield.some((c) => c.id === castingCardId) : false,
    [castingCardId, hand, battlefield],
  );

  // Track the chosen target so the arrow persists through cost payment
  const [targetId, setTargetId] = useState<string | null>(null);
  const [targetHostile, setTargetHostile] = useState(false);

  // Whether the engine says the current effect is hostile
  const promptHostile = currentPrompt?.hostile ?? true;

  // Clear target when casting card changes (new spell or cast finished)
  useEffect(() => {
    setTargetId(null);
    setTargetHostile(false);
  }, [castingCardId]);

  // Whether we're in the targeting phase (arrow follows cursor)
  const isTargeting = promptType?.startsWith("chooseTarget") ?? false;

  // Arrow hostility: use locked value if target chosen, else prompt value
  const arrowHostile = targetId ? targetHostile : promptHostile;

  // Wrapped target actions that record the chosen target
  const wrappedTargetCard = useCallback((cardId: string | null) => {
    if (cardId && castingCardId) {
      setTargetId(cardId);
      setTargetHostile(promptHostile);
    }
    targetCard(cardId);
  }, [targetCard, castingCardId, promptHostile]);

  const wrappedTargetPlayer = useCallback((playerId: string) => {
    if (castingCardId) {
      setTargetId(playerId);
      setTargetHostile(promptHostile);
    }
    targetPlayer(playerId);
  }, [targetPlayer, castingCardId, promptHostile]);

  const wrappedTargetAny = useCallback((target: { kind: string; playerId?: string; cardId?: string }) => {
    const id = target.cardId ?? target.playerId ?? null;
    if (castingCardId && id) {
      setTargetId(id);
      setTargetHostile(promptHostile);
    }
    targetAny(target);
  }, [targetAny, castingCardId, promptHostile]);

  return {
    /** The card ID being cast, or null. */
    castingCardId,
    /** The XMageCard object being cast, or null. */
    castingCard,
    /** Whether the source card is on the battlefield (activated ability) vs in hand. */
    castingFromBattlefield,
    /** Whether the casting arrow should follow the cursor (targeting phase). */
    isTargeting,
    /** The locked target ID after the player chose a target. */
    targetId,
    /** Whether the arrow should be hostile (red) or friendly (blue). */
    arrowHostile,
    /** Whether there's an active casting arrow to show. */
    showArrow: !!castingCardId && (isTargeting || !!targetId),
    /** Wrapped target actions that track the chosen target. */
    wrappedTargetCard,
    wrappedTargetPlayer,
    wrappedTargetAny,
  };
}
