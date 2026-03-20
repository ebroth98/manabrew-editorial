import { Ban, Check } from "lucide-react";
import { PromptActionButton } from "@/components/game/panels/PromptActionButton";
import { BUTTON_CONFIRM_BLOCKS, PROMPT_BUTTON_COLUMN } from "@/components/game/game.styles";
import { usePromptActionColors } from "@/components/game/panels/promptActionTheme";
import type { ChooseBlockersProps } from "./types";

export function ChooseBlockers({
  buttonLayout,
  isWaitingForResponse,
  pendingAttacker,
  blockAssignments,
  onPassPriority,
  onDeclareBlockers,
}: ChooseBlockersProps) {
  const promptActionColors = usePromptActionColors();
  const buttonGroupClass =
    buttonLayout === "modern"
      ? "flex flex-row flex-wrap items-center justify-center gap-3"
      : PROMPT_BUTTON_COLUMN;

  return (
    <div className={buttonGroupClass}>
      <PromptActionButton
        layout={buttonLayout}
        label="No Blockers"
        icon={<Ban className="h-3.5 w-3.5" />}
        variant="outline"
        baseColor={promptActionColors.pacificAction}
        onClick={onPassPriority}
        disabled={isWaitingForResponse}
      />
      {pendingAttacker && (
        <p className="text-xs italic text-muted-foreground">Attacker selected. Click your blocker.</p>
      )}
      {blockAssignments.length > 0 && (
        <PromptActionButton
          layout={buttonLayout}
          label={`Confirm Blocks (${blockAssignments.length})`}
          icon={<Check className="h-3.5 w-3.5" />}
          className={BUTTON_CONFIRM_BLOCKS}
          onClick={() => onDeclareBlockers(blockAssignments)}
          disabled={isWaitingForResponse}
        />
      )}
    </div>
  );
}
