import { CombatSummarySection } from "../CombatSummarySection";
import type { Card } from "@/types/openmagic";
import type { PromptActionType, CombatAssignment } from "../game.types";

interface CombatInfoProps {
  promptType?: PromptActionType;
  attackerIds: string[];
  pendingAttackers: string[];
  blockAssignments: CombatAssignment[];
  resolveCardName: (cardId: string) => string;
  resolveCard: (cardId: string) => Card | undefined;
}

export function CombatInfo({
  promptType,
  attackerIds,
  pendingAttackers,
  blockAssignments,
  resolveCardName,
  resolveCard,
}: CombatInfoProps) {
  return (
    <CombatSummarySection
      promptType={promptType}
      attackerIds={attackerIds}
      pendingAttackers={pendingAttackers}
      blockAssignments={blockAssignments}
      resolveCardName={resolveCardName}
      resolveCard={resolveCard}
    />
  );
}
