import { Ban, ChevronsRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { PromptActionButton } from "@/components/game/panels/PromptActionButton";
import { PROMPT_BUTTON_COLUMN } from "@/components/game/game.styles";
import {
  getPromptActionButtonStyle,
  usePromptActionColors,
} from "@/components/game/panels/promptActionTheme";
import type { ChooseActionProps } from "./types";

export function ChooseAction({
  buttonLayout,
  isWaitingForResponse,
  isMyTurn,
  passToPhaseShort,
  onPassPriority,
  onPassUntilEot,
}: ChooseActionProps) {
  const promptActionColors = usePromptActionColors();

  if (buttonLayout === "modern") {
    const passUntilLabel = isMyTurn ? "End Turn" : "Pass Till End";
    const passActionStyle = getPromptActionButtonStyle(promptActionColors.passAction);

    return (
      <div className="flex w-3/5 flex-col gap-1.5">
        <div className="relative group/pass-priority">
          <Button
            size="sm"
            variant="outline"
            className="h-9 w-full rounded-lg text-sm font-black tracking-[0.12em] !border-0 !text-white transition-[filter,box-shadow] hover:brightness-105"
            onClick={onPassPriority}
            disabled={isWaitingForResponse}
            style={passActionStyle}
          >
            PASS
          </Button>
          <span className="pointer-events-none absolute left-10 top-full mt-1 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded bg-transparent/80 px-2 py-0.5 text-[10px] font-semibold text-white opacity-0 transition-all duration-150 group-hover/pass-priority:translate-y-0 group-hover/pass-priority:opacity-100">
            {`Pass to ${passToPhaseShort}`}
          </span>
        </div>
        <div className="flex w-full mt-1 justify-end">
          <div className="relative group/pass-until">
            <Button
              size="sm"
              variant="outline"
              className="h-5 w-12 rounded-lg p-0 text-xs font-bold !border-0 !text-white transition-[filter,box-shadow] hover:brightness-105"
              onClick={onPassUntilEot}
              disabled={isWaitingForResponse}
              title={isMyTurn ? "End Turn (F6)" : "Pass Until Your Turn (F6)"}
              style={passActionStyle}
            >
              <ChevronsRight className="h-3 w-3" />
            </Button>
            <span className="pointer-events-none absolute right-0 top-1/2 -translate-y-[180%] whitespace-nowrap rounded bg-black/80 px-2 py-0.5 text-[10px] font-semibold text-white opacity-0 transition-opacity duration-150 delay-0 group-hover/pass-until:delay-150 group-hover/pass-until:opacity-100">
              {passUntilLabel}
            </span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={PROMPT_BUTTON_COLUMN}>
      <PromptActionButton
        layout={buttonLayout}
        label="Pass (Space)"
        icon={<Ban className="h-3.5 w-3.5" />}
        variant="outline"
        baseColor={promptActionColors.passAction}
        onClick={onPassPriority}
        disabled={isWaitingForResponse}
      />
      <PromptActionButton
        layout={buttonLayout}
        label={isMyTurn ? "End Turn (F6)" : "Pass Until Your Turn (F6)"}
        icon={<ChevronsRight className="h-3.5 w-3.5" />}
        variant="outline"
        baseColor={promptActionColors.passAction}
        onClick={onPassUntilEot}
        disabled={isWaitingForResponse}
        title="Pass priority to end of turn (F6)"
      />
    </div>
  );
}
