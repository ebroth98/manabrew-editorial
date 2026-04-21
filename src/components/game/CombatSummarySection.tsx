import type { PromptActionType, CombatAssignment } from "./game.types";
import { PromptType } from "@/types/promptType";

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
  if (promptType !== PromptType.ChooseBlockers || attackerIds.length === 0) return null;

  return (
    <div className="rounded-lg p-2 bg-destructive/10">
      <p className="text-xs font-semibold text-destructive mb-1">Combat</p>
      <div className="flex flex-col gap-0.5">
        {attackerIds.map((attackerId) => {
          const blockers = blockAssignments.filter((a) => a.attackerId === attackerId);
          const blockerNames = blockers.map((b) => resolveCardName(b.blockerId));
          return (
            <div key={attackerId} className="text-xs flex gap-1">
              <span className="font-semibold truncate">{resolveCardName(attackerId)}</span>
              <span className="text-muted-foreground">-&gt;</span>
              <span className={blockerNames.length === 0 ? "text-destructive italic" : "text-muted-foreground truncate"}>
                {blockerNames.length === 0 ? "unblocked" : blockerNames.join(", ")}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
