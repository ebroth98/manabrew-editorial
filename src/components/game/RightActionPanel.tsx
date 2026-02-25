import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import type { StackObject } from "@/types/xmage";
import { useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Settings,
  Sword,
  TimerOff,
} from "lucide-react";

const PROMPT_LABELS: Record<string, string> = {
  mulligan: "Keep this hand?",
  chooseAction: "Play a card or pass priority",
  chooseAttackers: "Declare attackers",
  chooseBlockers: "Declare blockers",
  chooseTargetPlayer: "Choose a target player",
  chooseTargetCard: "Choose a target creature",
  chooseTargetAny: "Choose a target (player or permanent)",
  chooseTargetCardFromZone: "Choose a target card from the zone",
  chooseTargetSpell: "Choose a spell on the stack to counter",
  chooseMode: "Choose a mode for the spell",
  chooseOptionalTrigger: "An optional ability would trigger",
  chooseKicker: "Pay the kicker cost?",
  chooseBuyback: "Pay the buyback cost?",
  chooseMultikicker: "Choose multikicker count",
  chooseReplicate: "Choose replicate count",
  chooseAlternativeCost: "Choose casting option",
  scry: "Scry: choose cards to put on the bottom",
  surveil: "Surveil: choose cards to send to graveyard",
  dig: "Dig: choose cards to take",
  chooseDiscard: "Discard cards",
  gameOver: "Game Over",
};

type PromptActionType =
  | "chooseAction"
  | "chooseAttackers"
  | "chooseBlockers"
  | "mulligan"
  | "chooseTargetPlayer"
  | "chooseTargetCard"
  | "chooseTargetAny"
  | "chooseTargetCardFromZone"
  | "chooseTargetSpell"
  | "chooseMode"
  | "chooseOptionalTrigger"
  | "chooseKicker"
  | "chooseBuyback"
  | "chooseMultikicker"
  | "chooseReplicate"
  | "chooseAlternativeCost"
  | "scry"
  | "surveil"
  | "dig"
  | "chooseDiscard"
  | "gameOver"
  | string;

interface CombatAssignment {
  blockerId: string;
  attackerId: string;
}

interface RightActionPanelProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
  promptType?: PromptActionType;
  isWaitingForResponse: boolean;
  isAutoPassing: boolean;
  availableAttackerIds: string[];
  pendingAttackers: string[];
  onPassPriority: () => void;
  onDeclareAttackers: (attackerIds: string[]) => void;
  pendingAttacker: string | null;
  attackerIds: string[];
  blockAssignments: CombatAssignment[];
  onDeclareBlockers: (assignments: CombatAssignment[]) => void;
  onMulliganDecision: (keep: boolean) => void;
  stack: StackObject[];
  onOpenStack: () => void;
  onConcede: () => void;
  resolveCardName: (cardId: string) => string;
  isMyPriority: boolean;
  turn: number;
  activePlayerName: string;
  isMyTurn: boolean;
  gameLog: string[];
}

function getPromptLabel(promptType?: string): string {
  if (!promptType) return "Waiting for your next decision";
  return PROMPT_LABELS[promptType] ?? promptType;
}

function PromptActionController({
  promptType,
  isWaitingForResponse,
  isAutoPassing,
  availableAttackerIds,
  pendingAttackers,
  onPassPriority,
  onDeclareAttackers,
  pendingAttacker,
  blockAssignments,
  onDeclareBlockers,
  onMulliganDecision,
  onOpenStack,
}: {
  promptType?: PromptActionType;
  isWaitingForResponse: boolean;
  isAutoPassing: boolean;
  availableAttackerIds: string[];
  pendingAttackers: string[];
  onPassPriority: () => void;
  onDeclareAttackers: (attackerIds: string[]) => void;
  pendingAttacker: string | null;
  blockAssignments: CombatAssignment[];
  onDeclareBlockers: (assignments: CombatAssignment[]) => void;
  onMulliganDecision: (keep: boolean) => void;
  onOpenStack: () => void;
}) {
  if (isAutoPassing) {
    return <p className="text-xs italic text-muted-foreground animate-pulse">Auto-passing...</p>;
  }

  switch (promptType) {
    case "chooseAction":
      return (
        <div className="flex flex-col gap-2">
          <Button size="sm" variant="outline" onClick={onPassPriority} disabled={isWaitingForResponse}>
            Pass (Space)
          </Button>
          <Button
            size="sm"
            variant="outline"
            className="flex items-center gap-1"
            onClick={onPassPriority}
            disabled={isWaitingForResponse}
            title="Pass priority to end of turn (F6)"
          >
            <TimerOff className="h-3.5 w-3.5" />
            End Turn (F6)
          </Button>
        </div>
      );

    case "chooseAttackers":
      return (
        <div className="flex flex-col gap-2">
          <Button size="sm" variant="outline" onClick={onPassPriority} disabled={isWaitingForResponse}>
            No Attackers
          </Button>
          <Button
            size="sm"
            variant="secondary"
            className="flex items-center gap-1"
            disabled={isWaitingForResponse}
            onClick={() => onDeclareAttackers(availableAttackerIds)}
          >
            <Sword className="h-3.5 w-3.5" />
            Attack All
          </Button>
          {pendingAttackers.length > 0 && (
            <Button
              size="sm"
              className="flex items-center gap-1 bg-orange-500 hover:bg-orange-600 text-white"
              disabled={isWaitingForResponse}
              onClick={() => onDeclareAttackers(pendingAttackers)}
            >
              <Sword className="h-3.5 w-3.5" />
              Attack ({pendingAttackers.length})
            </Button>
          )}
        </div>
      );

    case "chooseBlockers":
      return (
        <div className="flex flex-col gap-2">
          <Button size="sm" variant="outline" onClick={onPassPriority} disabled={isWaitingForResponse}>
            No Blockers
          </Button>
          {pendingAttacker && (
            <p className="text-xs italic text-muted-foreground">Attacker selected. Click your blocker.</p>
          )}
          {blockAssignments.length > 0 && (
            <Button
              size="sm"
              className="bg-blue-600 hover:bg-blue-700 text-white"
              disabled={isWaitingForResponse}
              onClick={() => onDeclareBlockers(blockAssignments)}
            >
              Confirm Blocks ({blockAssignments.length})
            </Button>
          )}
        </div>
      );

    case "mulligan":
      return (
        <div className="flex flex-col gap-2">
          <Button size="sm" onClick={() => onMulliganDecision(true)} disabled={isWaitingForResponse}>
            Keep Hand
          </Button>
          <Button
            size="sm"
            variant="destructive"
            onClick={() => onMulliganDecision(false)}
            disabled={isWaitingForResponse}
          >
            Mulligan
          </Button>
        </div>
      );

    case "chooseTargetSpell":
      return (
        <Button size="sm" onClick={onOpenStack} disabled={isWaitingForResponse}>
          Choose Counter Target
        </Button>
      );

    case "chooseTargetPlayer":
    case "chooseTargetCard":
    case "chooseTargetAny":
    case "chooseTargetCardFromZone":
      return <p className="text-xs text-muted-foreground">Select a highlighted target on the battlefield or in the selector.</p>;

    case "chooseMode":
    case "chooseOptionalTrigger":
    case "chooseKicker":
    case "chooseBuyback":
    case "chooseMultikicker":
    case "chooseReplicate":
    case "chooseAlternativeCost":
    case "scry":
    case "surveil":
    case "dig":
    case "chooseDiscard":
      return <p className="text-xs text-muted-foreground">Decision modal is open. Complete the prompt there.</p>;

    default:
      return <p className="text-xs text-muted-foreground">No action available for this state.</p>;
  }
}

function StackSection({
  stack,
  promptType,
  onOpenStack,
}: {
  stack: StackObject[];
  promptType?: PromptActionType;
  onOpenStack: () => void;
}) {
  const isCounterPrompt = promptType === "chooseTargetSpell";
  const show = stack.length > 0 || isCounterPrompt;

  if (!show) return null;

  return (
    <div className={cn(
      "rounded-lg p-2",
      isCounterPrompt ? "bg-blue-50 dark:bg-blue-950/20" : "bg-muted/20",
    )}>
      <div className="flex items-center justify-between gap-2">
        <p className={cn(
          "text-xs font-semibold",
          isCounterPrompt ? "text-blue-700 dark:text-blue-400" : "text-muted-foreground",
        )}>
          Stack ({stack.length})
        </p>
        <Button size="sm" variant="outline" className="h-6 px-2 text-xs" onClick={onOpenStack}>
          View
        </Button>
      </div>
      {stack.length > 0 && (
        <div className="mt-1 flex flex-col gap-0.5">
          {[...stack].reverse().slice(0, 5).map((obj, idx) => (
            <span key={obj.id} className="text-[11px] text-muted-foreground truncate">
              {idx === 0 ? "[TOP] " : ""}
              {obj.name}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

function CombatSummarySection({
  promptType,
  attackerIds,
  blockAssignments,
  resolveCardName,
}: {
  promptType?: PromptActionType;
  attackerIds: string[];
  blockAssignments: CombatAssignment[];
  resolveCardName: (cardId: string) => string;
}) {
  if (promptType !== "chooseBlockers" || attackerIds.length === 0) return null;

  return (
    <div className="rounded-lg p-2 bg-red-50 dark:bg-red-950/20">
      <p className="text-xs font-semibold text-red-700 dark:text-red-400 mb-1">Combat</p>
      <div className="flex flex-col gap-0.5">
        {attackerIds.map((attackerId) => {
          const blockers = blockAssignments.filter((a) => a.attackerId === attackerId);
          const blockerNames = blockers.map((b) => resolveCardName(b.blockerId));
          return (
            <div key={attackerId} className="text-xs flex gap-1">
              <span className="font-semibold truncate">{resolveCardName(attackerId)}</span>
              <span className="text-muted-foreground">-&gt;</span>
              <span className={blockerNames.length === 0 ? "text-red-500 italic" : "text-muted-foreground truncate"}>
                {blockerNames.length === 0 ? "unblocked" : blockerNames.join(", ")}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function TurnStateSection({
  turn,
  activePlayerName,
  isMyTurn,
  isMyPriority,
}: {
  turn: number;
  activePlayerName: string;
  isMyTurn: boolean;
  isMyPriority: boolean;
}) {
  return (
    <div className="rounded-lg px-2.5 py-2 bg-muted/25">
      <div className="flex items-center gap-1.5">
        <p className="text-sm font-semibold">Turn {turn} -</p>
        <p className={cn("text-sm font-medium", isMyTurn ? "text-green-700 dark:text-green-300" : "text-amber-700 dark:text-amber-300")}>
          {isMyTurn ? "Your turn" : `${activePlayerName}'s turn`}
        </p>
        {isMyPriority && (
          <span className="ml-1 text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0 bg-purple-100 text-purple-700 dark:bg-purple-950/40 dark:text-purple-300 animate-pulse">
            PRIORITY
          </span>
        )}
      </div>
    </div>
  );
}

export function RightActionPanel({
  collapsed,
  onToggleCollapse,
  promptType,
  isWaitingForResponse,
  isAutoPassing,
  availableAttackerIds,
  pendingAttackers,
  onPassPriority,
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
  const [activeTab, setActiveTab] = useState<"main" | "log">("main");
  const needsAction =
    Boolean(promptType) &&
    promptType !== "gameOver" &&
    !isWaitingForResponse &&
    !isAutoPassing;

  if (collapsed) {
    return (
      <aside
        className={cn(
          "w-12 shrink-0 rounded-lg bg-card/90 backdrop-blur-sm transition-colors overflow-hidden",
          needsAction && "bg-green-50/60 dark:bg-green-950/20 shadow-[inset_0_0_0_2px_rgba(34,197,94,0.85)]",
        )}
      >
        <div className="h-full w-full flex flex-col items-center justify-start py-2">
          <Button size="icon" variant="ghost" className="h-8 w-8" onClick={onToggleCollapse} title="Expand action panel">
            <ChevronLeft className="h-4 w-4" />
          </Button>
        </div>
      </aside>
    );
  }

  return (
    <aside
      className={cn(
        "w-72 shrink-0 rounded-lg bg-card/95 backdrop-blur-sm transition-colors overflow-hidden",
        needsAction && "bg-green-50/40 dark:bg-green-950/10 shadow-[inset_0_0_0_2px_rgba(34,197,94,0.85)]",
      )}
    >
      <div className="h-full p-3 flex flex-col gap-3 overflow-y-auto">
        <div className="flex items-end justify-between gap-2">
          <div className="flex items-center gap-5">
            <button
              className={cn(
                "h-8 text-xs font-semibold border-b-2 -mb-px transition-colors",
                activeTab === "main"
                  ? "text-foreground border-foreground"
                  : "text-muted-foreground border-transparent hover:text-foreground",
              )}
              onClick={() => setActiveTab("main")}
            >
              Main
            </button>
            <button
              className={cn(
                "h-8 text-xs font-semibold border-b-2 -mb-px transition-colors",
                activeTab === "log"
                  ? "text-foreground border-foreground"
                  : "text-muted-foreground border-transparent hover:text-foreground",
              )}
              onClick={() => setActiveTab("log")}
            >
              Log ({gameLog.length})
            </button>
          </div>
          <Button size="icon" variant="ghost" className="h-8 w-8 shrink-0" onClick={onToggleCollapse} title="Collapse action panel">
            <ChevronRight className="h-4 w-4" />
          </Button>
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
                availableAttackerIds={availableAttackerIds}
                pendingAttackers={pendingAttackers}
                onPassPriority={onPassPriority}
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
