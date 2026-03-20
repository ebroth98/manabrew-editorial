import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Settings } from "lucide-react";
import type { MainActionOverlayProps } from "../game.types";
import { PromptActionController } from "./PromptActionController";
import { CombatInfo } from "./CombatInfo";
import { PHASES } from "../game.constants";
import { PromptType } from "@/types/promptType";

export function MainActionOverlay({
  promptType,
  isWaitingForResponse,
  isAutoPassing,
  isPassingUntilEot,
  availableAttackerIds,
  pendingAttackers,
  onPassPriority,
  onPassUntilEot,
  onDeclareAttackers,
  pendingAttacker,
  attackerIds,
  blockAssignments,
  onDeclareBlockers,
  onOpenStack,
  onConcede,
  resolveCardName,
  isMyPriority: _isMyPriority,
  turn: _turn,
  activePlayerName: _activePlayerName,
  isMyTurn,
  step,
  payManaCostInfo,
  onPayManaCost,
  onCancelManaCost,
}: MainActionOverlayProps) {
  if (promptType === PromptType.GameOver) return null;
  const buttonLayout = "modern" as const;
  const currentPhaseIndex = PHASES.findIndex((phase) => phase.id === step);
  const passToPhaseShort =
    currentPhaseIndex >= 0
      ? PHASES[(currentPhaseIndex + 1) % PHASES.length]?.short ?? "NEXT"
      : "NEXT";

  return (
    <>
      <section className="absolute bottom-30 right-0 z-40 w-[300px] max-w-[calc(100%-12px)] flex flex-col gap-3 pb-[170px]">
        <CombatInfo
          promptType={promptType}
          attackerIds={attackerIds}
          blockAssignments={blockAssignments}
          resolveCardName={resolveCardName}
        />

        <div className="absolute bottom-0 right-0 w-[300px]">
          <div className="h-9" />
          <div className="flex flex-col items-center w-full [&_button]:mx-0">
            <PromptActionController
              promptType={promptType}
              isWaitingForResponse={isWaitingForResponse}
              isAutoPassing={isAutoPassing}
              isPassingUntilEot={isPassingUntilEot}
              isMyTurn={isMyTurn}
              passToPhaseShort={passToPhaseShort}
              availableAttackerIds={availableAttackerIds}
              pendingAttackers={pendingAttackers}
              onPassPriority={onPassPriority}
              onPassUntilEot={onPassUntilEot}
              onDeclareAttackers={onDeclareAttackers}
              pendingAttacker={pendingAttacker}
              blockAssignments={blockAssignments}
              onDeclareBlockers={onDeclareBlockers}
              onOpenStack={onOpenStack}
              buttonLayout={buttonLayout}
              payManaCostInfo={payManaCostInfo}
              onPayManaCost={onPayManaCost}
              onCancelManaCost={onCancelManaCost}
            />
          </div>
        </div>
      </section>

      <div className="absolute bottom-4 right-4 z-50">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="icon" variant="ghost" className="h-8 w-8 bg-black/35 hover:bg-black/55 text-white" title="Prompt options">
              <Settings className="h-3.5 w-3.5" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              className="text-destructive focus:text-destructive"
              onClick={onConcede}
            >
              Concede
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </>
  );
}
