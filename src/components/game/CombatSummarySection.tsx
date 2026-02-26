import type { PromptActionType, CombatAssignment } from "./game.types";

interface CombatSummarySectionProps {
  promptType?: PromptActionType;
  attackerIds: string[];
  blockAssignments: CombatAssignment[];
  resolveCardName: (cardId: string) => string;
}

export function CombatSummarySection({
  promptType,
  attackerIds,
  blockAssignments,
  resolveCardName,
}: CombatSummarySectionProps) {
  if (promptType !== "chooseBlockers" || attackerIds.length === 0) return null;

  return (
    <div className="rounded-lg p-2 bg-red-50 dark:bg-red-950/20">
      <p className="text-xs font-semibold text-red-700 dark:text-red-400 mb-1">Combat</p>
      <div className="flex flex-col gap-0.5">
        {attackerIds.map((attackerId) => {
          const blockers = blockAssignments.filter((a) => a.attackerId === attackerId);
          const blockerNames = blockers.map((b) => resolveCardName(b.blockerId));
          return (
            <div key={attackerId} className="text-xs flex gap-1">
              <span className="font-semibold truncate">{resolveCardName(attackerId)}</span>
              <span className="text-muted-foreground">-&gt;</span>
              <span className={blockerNames.length === 0 ? "text-red-500 italic" : "text-muted-foreground truncate"}>
                {blockerNames.length === 0 ? "unblocked" : blockerNames.join(", ")}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
