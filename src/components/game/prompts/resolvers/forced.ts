import type { PromptResolver, ResolveCtx } from "../promptHandlers";
import { useTargetIntentStore } from "@/stores/useTargetIntentStore";

function consumeIntentForCard(
  prompt: { sourceCardId?: string; validCardIds?: string[] },
  ctx: ResolveCtx,
): string | null {
  const sourceId = prompt.sourceCardId;
  if (!sourceId) return null;
  const intent = ctx.targetIntents[sourceId];
  if (!intent || intent.kind !== "card") return null;
  if (!(prompt.validCardIds ?? []).includes(intent.id)) return null;
  useTargetIntentStore.getState().clearIntent(sourceId);
  return intent.id;
}

function consumeIntentForPlayer(
  prompt: { sourceCardId?: string; validPlayerIds?: string[] },
  ctx: ResolveCtx,
): string | null {
  const sourceId = prompt.sourceCardId;
  if (!sourceId) return null;
  const intent = ctx.targetIntents[sourceId];
  if (!intent || intent.kind !== "player") return null;
  if (!(prompt.validPlayerIds ?? []).includes(intent.id)) return null;
  useTargetIntentStore.getState().clearIntent(sourceId);
  return intent.id;
}

function consumeIntentForSpell(
  prompt: { sourceCardId?: string; validSpellIds?: string[] },
  ctx: ResolveCtx,
): string | null {
  const sourceId = prompt.sourceCardId;
  if (!sourceId) return null;
  const intent = ctx.targetIntents[sourceId];
  if (!intent || intent.kind !== "spell") return null;
  if (!(prompt.validSpellIds ?? []).includes(intent.id)) return null;
  useTargetIntentStore.getState().clearIntent(sourceId);
  return intent.id;
}

export const singleLegalCard: PromptResolver = (prompt, ctx) => {
  const preselected = consumeIntentForCard(prompt, ctx);
  if (preselected) {
    return {
      kind: "auto",
      respond: { type: "targetCard", cardId: preselected },
      reason: `pre-selected target ${preselected}`,
    };
  }
  const ids = prompt.validCardIds ?? [];
  if (ids.length !== 1) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "targetCard", cardId: ids[0] },
    reason: `single legal target: ${ids[0]}`,
  };
};

export const singleLegalPlayer: PromptResolver = (prompt, ctx) => {
  const preselected = consumeIntentForPlayer(prompt, ctx);
  if (preselected) {
    return {
      kind: "auto",
      respond: { type: "targetPlayer", playerId: preselected },
      reason: `pre-selected player ${preselected}`,
    };
  }
  const ids = prompt.validPlayerIds ?? [];
  if (ids.length !== 1) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "targetPlayer", playerId: ids[0] },
    reason: `single legal player: ${ids[0]}`,
  };
};

export const singleLegalSpell: PromptResolver = (prompt, ctx) => {
  const preselected = consumeIntentForSpell(prompt, ctx);
  if (preselected) {
    return {
      kind: "auto",
      respond: { type: "targetSpell", spellId: preselected },
      reason: `pre-selected spell ${preselected}`,
    };
  }
  const ids = prompt.validSpellIds ?? [];
  if (ids.length !== 1) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "targetSpell", spellId: ids[0] },
    reason: `single legal spell: ${ids[0]}`,
  };
};

export const forcedAllModes: PromptResolver = (prompt) => {
  const opts = prompt.options ?? [];
  const min = prompt.minChoices ?? 0;
  const max = prompt.maxChoices ?? 0;
  if (opts.length === 0) return { kind: "force-show" };
  if (min !== max || min !== opts.length) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "modeDecision", chosenIndices: opts.map((_, i) => i) },
    reason: `must pick all ${opts.length} modes`,
  };
};

export const singleLegalColor: PromptResolver = (prompt) => {
  const colors = prompt.validColors ?? [];
  if (colors.length !== 1) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "colorDecision", color: colors[0] },
    reason: `only legal colour: ${colors[0]}`,
  };
};

export const singleLegalType: PromptResolver = (prompt) => {
  const types = prompt.validTypes ?? [];
  if (types.length !== 1) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "typeDecision", chosenType: types[0] },
    reason: `only legal type: ${types[0]}`,
  };
};

export const singleLegalNumber: PromptResolver = (prompt) => {
  const min = prompt.min;
  const max = prompt.max;
  if (min == null || max == null || min !== max) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "numberDecision", chosenNumber: min },
    reason: `only legal number: ${min}`,
  };
};

export const singleLegalName: PromptResolver = (prompt) => {
  const names = prompt.validNames ?? [];
  if (names.length !== 1) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "cardNameDecision", chosenName: names[0] },
    reason: `only legal name: ${names[0]}`,
  };
};

export const allCardsForced: PromptResolver = (prompt) => {
  const ids = prompt.validCardIds ?? prompt.cardIds ?? [];
  const min = prompt.minChoices ?? prompt.min;
  const max = prompt.maxChoices ?? prompt.max;
  if (ids.length === 0) return { kind: "force-show" };
  if (min == null || max == null || min !== max || min !== ids.length) {
    return { kind: "force-show" };
  }
  return {
    kind: "auto",
    respond: { type: "chooseCardsDecision", chosenCardIds: ids },
    reason: `must pick all ${ids.length} cards`,
  };
};

export const singleAlternativeCost: PromptResolver = (prompt) => {
  const opts = prompt.options ?? [];
  if (opts.length !== 1) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "alternativeCostDecision", chosenIndex: 0 },
    reason: "only one castable cost option",
  };
};

export const singleBlockerOrder: PromptResolver = (prompt) => {
  const blockers = prompt.blockerIds ?? [];
  if (blockers.length > 1) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "damageAssignmentOrderDecision", orderedBlockerIds: blockers },
    reason: `≤1 blocker (${blockers.length})`,
  };
};

export const singleAssigneeDamage: PromptResolver = (prompt) => {
  const blockers = prompt.blockerIds ?? [];
  const defenderId = prompt.defenderId ?? null;
  const total = prompt.totalDamage ?? 0;
  const assignees = blockers.length + (defenderId ? 1 : 0);
  if (assignees !== 1) return { kind: "force-show" };
  const target = blockers[0] ?? defenderId!;
  return {
    kind: "auto",
    respond: {
      type: "combatDamageAssignmentDecision",
      assignments: [{ assigneeId: target, damage: total }],
    },
    reason: `single assignee (${target}) gets all ${total} damage`,
  };
};

export const singleLegalAny: PromptResolver = (prompt, ctx) => {
  const preCard = consumeIntentForCard(prompt, ctx);
  if (preCard) {
    return {
      kind: "auto",
      respond: { type: "targetAny", target: { kind: "card", cardId: preCard } },
      reason: `pre-selected target (card ${preCard})`,
    };
  }
  const prePlayer = consumeIntentForPlayer(prompt, ctx);
  if (prePlayer) {
    return {
      kind: "auto",
      respond: { type: "targetAny", target: { kind: "player", playerId: prePlayer } },
      reason: `pre-selected target (player ${prePlayer})`,
    };
  }
  const preSpell = consumeIntentForSpell(prompt, ctx);
  if (preSpell) {
    return {
      kind: "auto",
      respond: { type: "targetAny", target: { kind: "stack", spellId: preSpell } },
      reason: `pre-selected target (spell ${preSpell})`,
    };
  }
  const cards = prompt.validCardIds ?? [];
  const players = prompt.validPlayerIds ?? [];
  const spells = prompt.validSpellIds ?? [];
  const total = cards.length + players.length + spells.length;
  if (total !== 1) return { kind: "force-show" };
  if (cards.length === 1) {
    return {
      kind: "auto",
      respond: { type: "targetAny", target: { kind: "card", cardId: cards[0] } },
      reason: `single legal target (card ${cards[0]})`,
    };
  }
  if (players.length === 1) {
    return {
      kind: "auto",
      respond: { type: "targetAny", target: { kind: "player", playerId: players[0] } },
      reason: `single legal target (player ${players[0]})`,
    };
  }
  return {
    kind: "auto",
    respond: { type: "targetAny", target: { kind: "stack", spellId: spells[0] } },
    reason: `single legal target (spell ${spells[0]})`,
  };
};

export const forcedDiscard: PromptResolver = (prompt) => {
  const hand = prompt.handCardIds ?? [];
  const required = prompt.numToDiscard ?? 0;
  if (required <= 0) {
    return {
      kind: "auto",
      respond: { type: "discardDecision", discardedCardIds: [] },
      reason: "discard 0 cards",
    };
  }
  if (required >= hand.length) {
    return {
      kind: "auto",
      respond: { type: "discardDecision", discardedCardIds: hand },
      reason: `discard entire hand (${hand.length} cards)`,
    };
  }
  return { kind: "force-show" };
};

export const emptyScry: PromptResolver = (prompt) => {
  const cards = prompt.cardIds ?? prompt.cards?.map((c) => c.id) ?? [];
  if (cards.length > 0) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "scryDecision", bottomCardIds: [] },
    reason: "scry with 0 revealed cards",
  };
};

export const emptySurveil: PromptResolver = (prompt) => {
  const cards = prompt.cardIds ?? prompt.cards?.map((c) => c.id) ?? [];
  if (cards.length > 0) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "surveilDecision", graveyardCardIds: [] },
    reason: "surveil with 0 revealed cards",
  };
};

export const emptyDig: PromptResolver = (prompt) => {
  const cards = prompt.cardIds ?? prompt.cards?.map((c) => c.id) ?? [];
  if (cards.length > 0) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "digDecision", chosenCardIds: [] },
    reason: "dig with 0 revealed cards",
  };
};

export const singleCardOrder: PromptResolver = (prompt) => {
  const ids = prompt.cardIds ?? prompt.cards?.map((c) => c.id) ?? [];
  if (ids.length > 1) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "reorderLibraryDecision", orderedCardIds: ids },
    reason: `≤1 card to order (${ids.length})`,
  };
};
