import { Check } from "lucide-react";
import { PromptActionButton } from "@/components/game/panels/PromptActionButton";
import type { ChooseTargetSpellProps } from "./types";

export function ChooseTargetSpell({
  buttonLayout,
  isWaitingForResponse,
  onOpenStack,
}: ChooseTargetSpellProps) {
  return (
    <PromptActionButton
      layout={buttonLayout}
      label="Choose Counter Target"
      icon={<Check className="h-3.5 w-3.5" />}
      onClick={onOpenStack}
      disabled={isWaitingForResponse}
    />
  );
}
