import { useState, useEffect, useCallback, useMemo } from "react";
import type { Card as XMageCard, StackObject } from "@/types/openmagic";
import type { AgentPrompt } from "@/stores/useGameStore";

/**
 * Prompt types that are part of the spell-casting flow.
 * While any of these are active, the casting card stays visible in the stack.
 */
/** Minimal XMageCard used when the engine's zones don't yet have the
 * spell we're casting — enough for StackDisplay to render the card face
 * and for the casting glow / target arrow to anchor to it. */
function stubCard(id: string, name: string, text = ""): XMageCard {
  return {
    id,
    name,
    setCode: "",
    cardNumber: "",
    color: "",
    manaCost: "",
    types: [],
    subtypes: [],
    supertypes: [],
    text,
    isPlayable: false,
    isSelected: false,
    isChoosable: false,
    controllerId: "",
    ownerId: "",
    zoneId: "stack",
  };
}

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
  /** Current stack so we can find the casting card once the engine has
   *  pushed it onto the stack (target prompts fire with the spell already
   *  on the stack for many effects). */
  stack?: StackObject[];
  /** Optional lookup by card id that covers cards the engine has already
   *  removed from every live zone (e.g. a spell in the "in-flight" state
   *  between leaving the hand and landing on the stack). Populate from a
   *  caller-side cache of every card ever observed. */
  resolveKnownCard?: (cardId: string) => XMageCard | undefined;
  targetCard: (cardId: string | null) => void;
  targetPlayer: (playerId: string) => void;
  targetAny: (target: { kind: string; playerId?: string; cardId?: string }) => void;
}

export function useCastingState({
  currentPrompt,
  hand,
  battlefield,
  stack,
  resolveKnownCard,
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

  // Resolve the actual card object. Order matters: the spell is usually
  // still in hand at the very start of the cast (cost prompts), but for
  // target prompts the engine may have already pulled it off the hand —
  // sometimes it's on the stack, sometimes it's in "limbo" (neither in a
  // zone nor yet pushed onto the stack). Fall through every source we have
  // and, as a last resort, synthesize a stub from the prompt itself so
  // the casting card always renders while a cast is in progress.
  const promptSourceCardName = currentPrompt?.sourceCardName;
  const castingCard = useMemo(() => {
    if (!castingCardId) return null;
    const fromHand = hand.find((c) => c.id === castingCardId);
    if (fromHand) return fromHand;
    const fromBattlefield = battlefield.find((c) => c.id === castingCardId);
    if (fromBattlefield) return fromBattlefield;
    // Prefer the caller-side "known cards" cache BEFORE the stack stub —
    // StackObject carries only name+text (no setCode / cardNumber), so a
    // stub would force a name-only Scryfall lookup and return a different
    // printing's art than the one originally in hand. The cache keeps the
    // exact card we've already been rendering.
    const cached = resolveKnownCard?.(castingCardId);
    if (cached) return cached;
    if (stack) {
      const stackEntry = stack.find((s) => s.sourceId === castingCardId);
      if (stackEntry) {
        return stubCard(stackEntry.sourceId, stackEntry.name, stackEntry.text);
      }
    }
    // Always render *something* while a cast is in progress so the player
    // has visual confirmation of what they're targeting.
    return stubCard(castingCardId, promptSourceCardName ?? "Casting…");
  }, [castingCardId, hand, battlefield, stack, promptSourceCardName, resolveKnownCard]);

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
