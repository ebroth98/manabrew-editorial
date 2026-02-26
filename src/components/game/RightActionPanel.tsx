import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useCallback, useState } from "react";
import { ChevronLeft, ChevronRight, Settings } from "lucide-react";
import type { RightActionPanelProps } from "./game.types";
import { getPromptLabel } from "./game.utils";
import { TAB_BUTTON_BASE, TAB_ACTIVE, TAB_INACTIVE } from "./game.styles";
import { useDragToggle } from "@/hooks/useDragToggle";
import { PromptActionController } from "./PromptActionController";
import { StackSection } from "./StackSection";
import { CombatSummarySection } from "./CombatSummarySection";
import { TurnStateSection } from "./TurnStateSection";

export function RightActionPanel({
  collapsed,
  onToggleCollapse: rawToggle,
  promptType,
  isWaitingForResponse,
  isAutoPassing,
  availableAttackerIds,
  pendingAttackers,
  onPassPriority,
  onPassUntilEot,
  isPassingUntilEot,
  onDeclareAttackers,
  pendingAttacker,
  attackerIds,
  blockAssignments,
  onDeclareBlockers,
  onMulliganDecision,
  stack,
  onOpenStack,
  onConcede,
  resolveCardName,
  isMyPriority,
  turn,
  activePlayerName,
  isMyTurn,
  gameLog,
}: RightActionPanelProps) {
  const expand = useCallback(() => { if (collapsed) rawToggle(); }, [collapsed, rawToggle]);
  const collapse = useCallback(() => { if (!collapsed) rawToggle(); }, [collapsed, rawToggle]);
  const onDragMouseDown = useDragToggle(expand, collapse, "left");

  const [activeTab, setActiveTab] = useState<"main" | "log">("main");
  const needsAction =
    Boolean(promptType) &&
    promptType !== "gameOver" &&
    !isWaitingForResponse &&
    !isAutoPassing &&
    !isPassingUntilEot;

  const edgeButtonClass = cn(
    "h-24 w-4 rounded-l-md rounded-r-none border border-r-0 border-border bg-card/90 px-0",
    "translate-x-[9px] group-hover:translate-x-0 group-hover:w-6 group-hover:h-28 transition-all duration-150",
    "hover:bg-card",
  );

  if (collapsed) {
    return (
      <aside
        className={cn(
          "relative w-12 shrink-0 rounded-lg bg-card/90 backdrop-blur-sm transition-colors overflow-visible",
          needsAction && "bg-green-50/60 dark:bg-green-950/20 shadow-[inset_0_0_0_2px_rgba(34,197,94,0.85)]",
        )}
      >
        <div className="absolute left-0 top-1/2 -translate-y-1/2 -translate-x-full z-30 group">
          <Button
            size="icon"
            variant="ghost"
            className={edgeButtonClass}
            onClick={rawToggle}
            onMouseDown={onDragMouseDown}
            title="Expand action panel"
          >
            <ChevronLeft className="h-3 w-3" />
          </Button>
        </div>
      </aside>
    );
  }

  return (
    <aside
      className={cn(
        "relative w-72 shrink-0 rounded-lg bg-card/95 backdrop-blur-sm transition-colors overflow-visible",
        needsAction && "bg-green-50/40 dark:bg-green-950/10 shadow-[inset_0_0_0_2px_rgba(34,197,94,0.85)]",
      )}
    >
      <div className="absolute left-0 top-1/2 -translate-y-1/2 -translate-x-full z-30 group">
        <Button
          size="icon"
          variant="ghost"
          className={edgeButtonClass}
          onClick={rawToggle}
          onMouseDown={onDragMouseDown}
          title="Collapse action panel"
        >
          <ChevronRight className="h-3 w-3" />
        </Button>
      </div>
      <div className="h-full p-3 flex flex-col gap-3 overflow-y-auto">
        <div className="flex items-center gap-5">
          <button
            className={cn(TAB_BUTTON_BASE, activeTab === "main" ? TAB_ACTIVE : TAB_INACTIVE)}
            onClick={() => setActiveTab("main")}
          >
            Main
          </button>
          <button
            className={cn(TAB_BUTTON_BASE, activeTab === "log" ? TAB_ACTIVE : TAB_INACTIVE)}
            onClick={() => setActiveTab("log")}
          >
            Log ({gameLog.length})
          </button>
        </div>

        {activeTab === "main" ? (
          <>
            <TurnStateSection
              turn={turn}
              activePlayerName={activePlayerName}
              isMyTurn={isMyTurn}
              isMyPriority={isMyPriority}
            />

            {isWaitingForResponse && (
              <p className="text-xs italic text-muted-foreground animate-pulse">Waiting for response...</p>
            )}

            <StackSection stack={stack} promptType={promptType} onOpenStack={onOpenStack} />

            <CombatSummarySection
              promptType={promptType}
              attackerIds={attackerIds}
              blockAssignments={blockAssignments}
              resolveCardName={resolveCardName}
            />

            <div
              className={cn(
                "rounded-lg p-2.5 mt-auto",
                isMyPriority
                  ? "bg-purple-50/70 dark:bg-purple-950/25 shadow-[inset_0_0_0_1px_rgba(168,85,247,0.45)]"
                  : "bg-muted/20",
              )}
            >
              <div className="flex items-center justify-between gap-2 mb-2">
                <p className="text-xs font-semibold text-muted-foreground">{getPromptLabel(promptType)}</p>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button size="icon" variant="ghost" className="h-6 w-6" title="Prompt options">
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
              <PromptActionController
                promptType={promptType}
                isWaitingForResponse={isWaitingForResponse}
                isAutoPassing={isAutoPassing}
                isPassingUntilEot={isPassingUntilEot}
                isMyTurn={isMyTurn}
                availableAttackerIds={availableAttackerIds}
                pendingAttackers={pendingAttackers}
                onPassPriority={onPassPriority}
                onPassUntilEot={onPassUntilEot}
                onDeclareAttackers={onDeclareAttackers}
                pendingAttacker={pendingAttacker}
                blockAssignments={blockAssignments}
                onDeclareBlockers={onDeclareBlockers}
                onMulliganDecision={onMulliganDecision}
                onOpenStack={onOpenStack}
              />
            </div>
          </>
        ) : (
          <div className="rounded-lg p-2.5 min-h-0 flex-1 flex flex-col bg-muted/20">
            <p className="text-xs font-semibold text-muted-foreground mb-2">Game Log</p>
            {gameLog.length === 0 ? (
              <p className="text-xs text-muted-foreground italic">No log entries yet.</p>
            ) : (
              <div className="min-h-0 flex-1 overflow-y-auto text-xs text-muted-foreground flex flex-col-reverse pr-1">
                {gameLog.slice(-200).reverse().map((msg, i) => (
                  <div key={i} className="py-0.5 border-b border-border/40 last:border-b-0">
                    {msg}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </aside>
  );
}
