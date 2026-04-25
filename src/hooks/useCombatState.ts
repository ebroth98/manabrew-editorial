import { useEffect, useState } from "react";
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
  currentPrompt,
}: UseCombatStateOptions) {
  const [pendingAttackers, setPendingAttackers] = useState<string[]>([]);
  const [pendingAttacker, setPendingAttacker] = useState<string | null>(null);
  const [attackDefenderId, setAttackDefenderId] = useState<string | null>(null);
  const [blockAssignments, setBlockAssignments] = useState<CombatAssignment[]>([]);

  // Reset combat state whenever the prompt type changes
  useEffect(() => {
    setPendingAttackers([]);
    setPendingAttacker(null);
    setAttackDefenderId(null);
    setBlockAssignments([]);
  }, [promptType]);

  useEffect(() => {
    if (promptType !== PromptType.ChooseAttackers) return;
    const defenders = currentPrompt?.possibleDefenderIds ?? [];
    if (defenders.length === 0) return;
    if (!attackDefenderId || !defenders.some((defender) => defender.id === attackDefenderId)) {
      setAttackDefenderId(defenders[0]!.id);
    }
  }, [attackDefenderId, currentPrompt?.possibleDefenderIds, promptType]);

  const playerIsTargetable =
    promptType === PromptType.ChooseAttackers
      ? (pid: string) =>
          currentPrompt?.possibleDefenderIds?.some((defender) => defender.id === pid) ?? false
      : promptType === PromptType.ChooseTargetPlayer || promptType === PromptType.ChooseTargetAny
        ? (pid: string) => currentPrompt?.validPlayerIds?.includes(pid) ?? false
        : () => false;

  function handleTargetPlayer(pid: string) {
    if (promptType === PromptType.ChooseAttackers) {
      setAttackDefenderId(pid);
    } else if (promptType === PromptType.ChooseTargetAny) {
      targetAny({ kind: "player", playerId: pid });
    } else {
      targetPlayer(pid);
    }
  }

  function handleBattlefieldClick(card: Card) {
    if (!currentPrompt || !card.isChoosable) return;

    if (promptType === PromptType.ChooseAttackers) {
      setPendingAttackers((prev) =>
        prev.includes(card.id) ? prev.filter((id) => id !== card.id) : [...prev, card.id],
      );
    } else if (promptType === PromptType.ChooseBlockers) {
      if (pendingAttacker) {
        setBlockAssignments((prev) => {
          const rest = prev.filter((a) => a.attackerId !== pendingAttacker);
          return [...rest, { blockerId: card.id, attackerId: pendingAttacker }];
        });
        setPendingAttacker(null);
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
    playerIsTargetable,
    handleTargetPlayer,
    handleBattlefieldClick,
    handleAttackerClick,
  };
}
