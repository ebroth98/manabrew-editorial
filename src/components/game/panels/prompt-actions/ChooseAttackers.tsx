import { Ban, Sword } from "lucide-react";
import { Button } from "@/components/ui/button";
import { PromptActionButton } from "@/components/game/panels/PromptActionButton";
import { BUTTON_ATTACK, PROMPT_BUTTON_COLUMN } from "@/components/game/game.styles";
import {
  getPromptActionButtonStyle,
  usePromptActionColors,
} from "@/components/game/panels/promptActionTheme";
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
  const hasPendingAttackers = pendingAttackers.length > 0;

  if (buttonLayout === "modern") {
    const attackAllStyle = getPromptActionButtonStyle(promptActionColors.attackAction);
    const attackStyle = getPromptActionButtonStyle(promptActionColors.attackAction);
    const passStyle = getPromptActionButtonStyle(promptActionColors.passAction);

    return (
      <div className="flex w-3/5 flex-col gap-1.5">
        <Button
          size="sm"
          variant="outline"
          className="h-9 w-full rounded-lg text-sm font-black tracking-[0.12em] !border-0 !text-white transition-[filter,box-shadow] hover:brightness-105"
          onClick={() => onDeclareAttackers(availableAttackerIds)}
          disabled={isWaitingForResponse}
          style={attackAllStyle}
        >
          ATTACK ALL
        </Button>
        <Button
          size="sm"
          variant="outline"
          className="h-9 w-full rounded-lg text-sm font-black tracking-[0.12em] !border-0 !text-white transition-[filter,box-shadow] hover:brightness-105"
          onClick={() => onDeclareAttackers(pendingAttackers)}
          disabled={isWaitingForResponse || !hasPendingAttackers}
          style={attackStyle}
        >
          {hasPendingAttackers ? `ATTACK (${pendingAttackers.length})` : "ATTACK"}
        </Button>
        <Button
          size="sm"
          variant="outline"
          className="h-9 w-full rounded-lg text-sm font-black tracking-[0.12em] !border-0 !text-white transition-[filter,box-shadow] hover:brightness-105"
          onClick={onPassPriority}
          disabled={isWaitingForResponse}
          style={passStyle}
        >
          PASS
        </Button>
      </div>
    );
  }

  return (
    <div className={PROMPT_BUTTON_COLUMN}>
      <PromptActionButton
        layout={buttonLayout}
        label="Attack All"
        icon={<Sword className="h-3.5 w-3.5" />}
        variant="secondary"
        baseColor={promptActionColors.attackAction}
        onClick={() => onDeclareAttackers(availableAttackerIds)}
        disabled={isWaitingForResponse}
      />
      <PromptActionButton
        layout={buttonLayout}
        label={hasPendingAttackers ? `Attack (${pendingAttackers.length})` : "Attack"}
        icon={<Sword className="h-3.5 w-3.5" />}
        className={BUTTON_ATTACK}
        baseColor={promptActionColors.attackAction}
        onClick={() => onDeclareAttackers(pendingAttackers)}
        disabled={isWaitingForResponse || !hasPendingAttackers}
      />
      <PromptActionButton
        layout={buttonLayout}
        label="Pass"
        icon={<Ban className="h-3.5 w-3.5" />}
        variant="outline"
        baseColor={promptActionColors.passAction}
        onClick={onPassPriority}
        disabled={isWaitingForResponse}
      />
    </div>
  );
}
