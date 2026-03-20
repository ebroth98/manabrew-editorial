import { Ban, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { PromptActionButton } from "@/components/game/panels/PromptActionButton";
import { BUTTON_CONFIRM_BLOCKS, PROMPT_BUTTON_COLUMN } from "@/components/game/game.styles";
import {
  getPromptActionButtonStyle,
  usePromptActionColors,
} from "@/components/game/panels/promptActionTheme";
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

  if (buttonLayout === "modern") {
    const defenseStyle = getPromptActionButtonStyle(promptActionColors.defenseAction);

    return (
      <div className="flex w-3/5 flex-col gap-1.5">
        <Button
          size="sm"
          variant="outline"
          className="h-9 w-full rounded-lg text-sm font-black tracking-[0.12em] !border-0 !text-white transition-[filter,box-shadow] hover:brightness-105"
          onClick={onPassPriority}
          disabled={isWaitingForResponse}
          style={defenseStyle}
        >
          NO BLOCKERS
        </Button>
        {pendingAttacker && (
          <p className="text-xs italic text-muted-foreground text-center">Attacker selected. Click your blocker.</p>
        )}
        {blockAssignments.length > 0 && (
          <Button
            size="sm"
            variant="outline"
            className="h-9 w-full rounded-lg text-sm font-black tracking-[0.12em] !border-0 !text-white transition-[filter,box-shadow] hover:brightness-105"
            onClick={() => onDeclareBlockers(blockAssignments)}
            disabled={isWaitingForResponse}
            style={defenseStyle}
          >
            {`CONFIRM BLOCKS (${blockAssignments.length})`}
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className={PROMPT_BUTTON_COLUMN}>
      <PromptActionButton
        layout={buttonLayout}
        label="No Blockers"
        icon={<Ban className="h-3.5 w-3.5" />}
        variant="outline"
        baseColor={promptActionColors.defenseAction}
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
          baseColor={promptActionColors.defenseAction}
          onClick={() => onDeclareBlockers(blockAssignments)}
          disabled={isWaitingForResponse}
        />
      )}
    </div>
  );
}
