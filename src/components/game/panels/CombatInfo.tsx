import { CombatSummarySection } from "../CombatSummarySection";
import type { PromptActionType, CombatAssignment } from "../game.types";

interface CombatInfoProps {
  promptType?: PromptActionType;
  attackerIds: string[];
  blockAssignments: CombatAssignment[];
  resolveCardName: (cardId: string) => string;
}

/**
 * CombatInfo displays combat phase information including attackers and blockers.
 * This is a thin wrapper around CombatSummarySection for organizational purposes.
 */
export function CombatInfo({
  promptType,
  attackerIds,
  blockAssignments,
  resolveCardName,
}: CombatInfoProps) {
  return (
    <CombatSummarySection
      promptType={promptType}
      attackerIds={attackerIds}
      blockAssignments={blockAssignments}
      resolveCardName={resolveCardName}
    />
  );
}
