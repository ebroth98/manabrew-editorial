import { Ban, Sword } from "lucide-react";
import { PromptActionButton } from "@/components/game/panels/PromptActionButton";
import { BUTTON_ATTACK, PROMPT_BUTTON_COLUMN } from "@/components/game/game.styles";
import { usePromptActionColors } from "@/components/game/panels/promptActionTheme";
import type { ChooseAttackersProps } from "./types";

export function ChooseAttackers({
  buttonLayout,
  isWaitingForResponse,
  availableAttackerIds,
  pendingAttackers,
  onPassPriority,
  onDeclareAttackers,
}: ChooseAttackersProps) {
  const promptActionColors = usePromptActionColors();
  const buttonGroupClass =
    buttonLayout === "modern"
      ? "flex flex-row flex-wrap items-center justify-center gap-3"
      : PROMPT_BUTTON_COLUMN;

  return (
    <div className={buttonGroupClass}>
      <PromptActionButton
        layout={buttonLayout}
        label="No Attackers"
        icon={<Ban className="h-3.5 w-3.5" />}
        variant="outline"
        baseColor={promptActionColors.pacificAction}
        onClick={onPassPriority}
        disabled={isWaitingForResponse}
      />
      <PromptActionButton
        layout={buttonLayout}
        label="Attack All"
        icon={<Sword className="h-3.5 w-3.5" />}
        variant="secondary"
        onClick={() => onDeclareAttackers(availableAttackerIds)}
        disabled={isWaitingForResponse}
      />
      {pendingAttackers.length > 0 && (
        <PromptActionButton
          layout={buttonLayout}
          label={`Attack (${pendingAttackers.length})`}
          icon={<Sword className="h-3.5 w-3.5" />}
          className={BUTTON_ATTACK}
          onClick={() => onDeclareAttackers(pendingAttackers)}
          disabled={isWaitingForResponse}
        />
      )}
    </div>
  );
}
