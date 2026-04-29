import { useState } from "react";
import type { Card } from "@/types/openmagic";
import { PromptType } from "@/types/promptType";

export interface CombatAssignment {
  blockerId: string;
  attackerId: string;
}

interface UseCombatStateOptions {
  promptType: string | undefined;
  targetCard: (cardId: string) => void;
  targetAny: (target: { kind: string; playerId?: string; cardId?: string }) => void;
  targetPlayer: (playerId: string) => void;
  /** Final commit for the multi-defender flow — invoked once the user
   *  has clicked a player avatar or defender card. */
  declareAttackers: (attackerIds: string[], defenderId?: string) => void;
  currentPrompt: {
    validPlayerIds?: string[];
    possibleDefenderIds?: { id: string; label: string }[];
  } | null;
}

export function useCombatState({
  promptType,
  targetCard,
  targetAny,
  targetPlayer,
  declareAttackers,
  currentPrompt,
}: UseCombatStateOptions) {
  const [pendingAttackers, setPendingAttackers] = useState<string[]>([]);
  const [pendingAttacker, setPendingAttacker] = useState<string | null>(null);
  const [attackDefenderId, setAttackDefenderId] = useState<string | null>(null);
  const [blockAssignments, setBlockAssignments] = useState<CombatAssignment[]>([]);

  // Reset combat state whenever the prompt type changes
  const [prevPromptType, setPrevPromptType] = useState(promptType);
  if (prevPromptType !== promptType) {
    setPrevPromptType(promptType);
    setPendingAttackers([]);
    setPendingAttacker(null);
    setAttackDefenderId(null);
    setBlockAssignments([]);
  }

  const possibleDefenders = currentPrompt?.possibleDefenderIds ?? [];
  const multipleAttackDefenders = possibleDefenders.length > 1;

  // Awaiting-defender state is implicit now: as soon as the user has at
  // least one pending attacker AND there's more than one legal defender
  // (multiplayer / planeswalkers / sieges), the next click on a valid
  // defender commits the whole pending batch against it.
  const awaitingAttackTarget =
    promptType === PromptType.ChooseAttackers &&
    multipleAttackDefenders &&
    pendingAttackers.length > 0;

  // Default attackDefenderId to first valid defender during ChooseAttackers.
  if (promptType === PromptType.ChooseAttackers) {
    if (
      possibleDefenders.length > 0 &&
      (!attackDefenderId || !possibleDefenders.some((d) => d.id === attackDefenderId))
    ) {
      const next = possibleDefenders[0]!.id;
      if (next !== attackDefenderId) setAttackDefenderId(next);
    }
  }

  const playerIsTargetable =
    promptType === PromptType.ChooseAttackers
      ? (pid: string) => possibleDefenders.some((defender) => defender.id === pid)
      : promptType === PromptType.ChooseTargetPlayer || promptType === PromptType.ChooseTargetAny
        ? (pid: string) => currentPrompt?.validPlayerIds?.includes(pid) ?? false
        : () => false;

  /** True when a battlefield card is a legal defender (planeswalker /
   *  siege) — shown choosable so battlefield clicks land on it. */
  function cardIsAttackTarget(cardId: string): boolean {
    return awaitingAttackTarget && possibleDefenders.some((defender) => defender.id === cardId);
  }

  function commitAttackAgainst(defenderId: string) {
    if (pendingAttackers.length === 0) return;
    declareAttackers(pendingAttackers, defenderId);
    setPendingAttackers([]);
  }

  /** "Attack All" — mark every legal attacker as pending. In single-
   *  defender games this commits immediately; in multi-defender games
   *  it leaves the attackers tapped and waiting for the user to click
   *  a target. */
  function selectAllAttackersForPick(attackerIds: string[]) {
    if (attackerIds.length === 0) return;
    if (possibleDefenders.length <= 1) {
      declareAttackers(attackerIds, possibleDefenders[0]?.id);
      return;
    }
    setPendingAttackers(attackerIds);
  }

  function cancelAttackTargetPick() {
    setPendingAttackers([]);
  }

  function handleTargetPlayer(pid: string) {
    if (awaitingAttackTarget && possibleDefenders.some((d) => d.id === pid)) {
      commitAttackAgainst(pid);
      return;
    }
    if (promptType === PromptType.ChooseAttackers) {
      setAttackDefenderId(pid);
    } else if (promptType === PromptType.ChooseTargetAny) {
      targetAny({ kind: "player", playerId: pid });
    } else {
      targetPlayer(pid);
    }
  }

  function handleBattlefieldClick(card: Card) {
    if (!currentPrompt) return;

    // Awaiting a defender pick — battlefield cards can be defenders too
    // (planeswalkers, sieges). Bypass `isChoosable` because the engine
    // doesn't pre-mark defender cards during attacker declaration; we
    // gate on `possibleDefenderIds` instead.
    if (awaitingAttackTarget && possibleDefenders.some((d) => d.id === card.id)) {
      commitAttackAgainst(card.id);
      return;
    }

    if (!card.isChoosable) return;

    if (promptType === PromptType.ChooseAttackers) {
      setPendingAttackers((prev) =>
        prev.includes(card.id) ? prev.filter((id) => id !== card.id) : [...prev, card.id],
      );
    } else if (promptType === PromptType.ChooseBlockers) {
      if (pendingAttacker) {
        setBlockAssignments((prev) => {
          // Toggle: clicking the same blocker on the same attacker again
          // unassigns it.
          const alreadyOnAttacker = prev.some(
            (a) => a.blockerId === card.id && a.attackerId === pendingAttacker,
          );
          if (alreadyOnAttacker) {
            return prev.filter(
              (a) => !(a.blockerId === card.id && a.attackerId === pendingAttacker),
            );
          }
          // MTG 509.1c — each creature can block at most one attacker. If
          // this blocker was already assigned elsewhere, move it (don't
          // duplicate). We deliberately do NOT strip other blockers off
          // the current attacker — multiple creatures can block one
          // attacker, and the engine handles legality (Menace etc).
          const withoutBlocker = prev.filter((a) => a.blockerId !== card.id);
          return [...withoutBlocker, { blockerId: card.id, attackerId: pendingAttacker }];
        });
        // Keep `pendingAttacker` selected so the user can chain multiple
        // blockers onto the same attacker without re-clicking it.
      }
    } else if (promptType === PromptType.ChooseTargetCard) {
      targetCard(card.id);
    } else if (promptType === PromptType.ChooseTargetAny) {
      targetAny({ kind: "card", cardId: card.id });
    }
  }

  function handleAttackerClick(card: Card) {
    setPendingAttacker((prev) => (prev === card.id ? null : card.id));
  }

  return {
    pendingAttackers,
    pendingAttacker,
    attackDefenderId,
    blockAssignments,
    multipleAttackDefenders,
    awaitingAttackTarget,
    playerIsTargetable,
    cardIsAttackTarget,
    handleTargetPlayer,
    handleBattlefieldClick,
    handleAttackerClick,
    selectAllAttackersForPick,
    cancelAttackTargetPick,
  };
}
