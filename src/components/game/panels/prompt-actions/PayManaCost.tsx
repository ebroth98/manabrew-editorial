import { Ban, Check } from "lucide-react";
import { TextWithMana } from "@/components/game/TextWithMana";
import { ManaPool } from "@/components/game/panels/ManaPool";
import { PROMPT_BUTTON_COLUMN } from "@/components/game/game.styles";
import { PromptActionButton } from "@/components/game/panels/PromptActionButton";
import { usePromptActionColors } from "@/components/game/panels/promptActionTheme";
import type { PayManaCostProps } from "./types";

export function PayManaCost({
  buttonLayout,
  isWaitingForResponse,
  payManaCostInfo,
  onPayManaCost,
  onCancelManaCost,
}: PayManaCostProps) {
  const promptActionColors = usePromptActionColors();
  const buttonGroupClass =
    buttonLayout === "modern"
      ? "flex flex-row flex-wrap items-center justify-center gap-3"
      : PROMPT_BUTTON_COLUMN;

  return (
    <div className={PROMPT_BUTTON_COLUMN}>
      {payManaCostInfo && (
        <>
          <p className="text-xs text-muted-foreground">
            Cast <span className="font-semibold text-foreground">{payManaCostInfo.cardName}</span>{" "}
            for <TextWithMana text={payManaCostInfo.manaCost} manaSize="sm" />
          </p>
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>Mana pool:</span>
            <ManaPool pool={payManaCostInfo.manaPool} />
          </div>
          <p className="text-[11px] text-muted-foreground/70">
            Tap lands to generate mana, then click Pay.
          </p>
        </>
      )}
      <div className={buttonGroupClass}>
        <PromptActionButton
          layout={buttonLayout}
          label="Pay"
          icon={<Check className="h-3.5 w-3.5" />}
          onClick={onPayManaCost}
          disabled={isWaitingForResponse}
        />
        <PromptActionButton
          layout={buttonLayout}
          label="Cancel"
          icon={<Ban className="h-3.5 w-3.5" />}
          variant="outline"
          baseColor={promptActionColors.cancel}
          onClick={onCancelManaCost}
          disabled={isWaitingForResponse}
        />
      </div>
    </div>
  );
}
