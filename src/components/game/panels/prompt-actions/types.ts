import type { CombatAssignment } from "@/components/game/game.types";
import type { PromptButtonLayout } from "@/components/game/panels/PromptActionButton";

export interface PromptActionLayoutProps {
  buttonLayout: PromptButtonLayout;
  isWaitingForResponse: boolean;
}

export interface NoActionProps {
  buttonLayout: PromptButtonLayout;
  label?: string;
}

export interface ChooseActionProps extends PromptActionLayoutProps {
  isMyTurn: boolean;
  passToPhaseShort: string;
  onPassPriority: () => void;
  onPassUntilEot: () => void;
}

export interface ChooseAttackersProps extends PromptActionLayoutProps {
  availableAttackerIds: string[];
  pendingAttackers: string[];
  onPassPriority: () => void;
  onDeclareAttackers: (attackerIds: string[]) => void;
}

export interface ChooseBlockersProps extends PromptActionLayoutProps {
  pendingAttacker: string | null;
  blockAssignments: CombatAssignment[];
  onPassPriority: () => void;
  onDeclareBlockers: (assignments: CombatAssignment[]) => void;
}

export interface ChooseTargetSpellProps extends PromptActionLayoutProps {
  onOpenStack: () => void;
}

export interface PayManaCostProps extends PromptActionLayoutProps {
  payManaCostInfo?: { cardName: string; manaCost: string; manaPool: Record<string, number> } | null;
  onPayManaCost?: () => void;
  onCancelManaCost?: () => void;
}

export interface PromptRequiredProps extends PromptActionLayoutProps {
  hidden: boolean;
  onOpenPrompt: () => void;
}
