import { PromptType } from "@/types/promptType";
import type { AgentPrompt } from "@/stores/gameStore.types";

export type RespondPayload = Record<string, unknown>;

export type AutoResolution =
  | {
      kind: "auto";
      respond: RespondPayload;
      reason: string;
    }
  | { kind: "force-show" };

export interface ResolveCtx {
  prefs: PromptPreferencesSnapshot;
  targetIntents: Record<string, { kind: "card" | "player" | "spell"; id: string }>;
}

export interface PromptPreferencesSnapshot {
  show: Partial<Record<PromptType, boolean>>;
  triggerMemory: Record<string, "yes" | "no">;
}

export type PromptResolver = (prompt: AgentPrompt, ctx: ResolveCtx) => AutoResolution;

export interface PromptHandler {
  showByDefault: boolean;
  resolve?: PromptResolver;
}

import * as informational from "./resolvers/informational";
import * as forced from "./resolvers/forced";
import * as optionalCosts from "./resolvers/optionalCosts";
import * as triggerMemory from "./resolvers/triggerMemory";

export const PROMPT_HANDLERS: Record<PromptType, PromptHandler> = {
  [PromptType.StateUpdate]: { showByDefault: false },
  [PromptType.GameOver]: { showByDefault: true },

  [PromptType.Mulligan]: { showByDefault: true },
  [PromptType.MulliganPutBack]: { showByDefault: true },

  [PromptType.ChooseAction]: { showByDefault: false },

  [PromptType.ChooseAttackers]: { showByDefault: false },
  [PromptType.ChooseBlockers]: { showByDefault: false },
  [PromptType.ChooseExertAttackers]: {
    showByDefault: true,
    resolve: optionalCosts.skipExertEnlist,
  },
  [PromptType.ChooseEnlistAttackers]: {
    showByDefault: true,
    resolve: optionalCosts.skipExertEnlist,
  },
  [PromptType.ChooseDamageAssignmentOrder]: {
    showByDefault: true,
    resolve: forced.singleBlockerOrder,
  },
  [PromptType.ChooseCombatDamageAssignment]: {
    showByDefault: true,
    resolve: forced.singleAssigneeDamage,
  },
  [PromptType.PayCombatCost]: { showByDefault: true },

  [PromptType.ChooseTargetCard]: { showByDefault: true, resolve: forced.singleLegalCard },
  [PromptType.ChooseTargetCardFromZone]: { showByDefault: true, resolve: forced.singleLegalCard },
  [PromptType.ChooseTargetPlayer]: { showByDefault: true, resolve: forced.singleLegalPlayer },
  [PromptType.ChooseTargetAny]: { showByDefault: true, resolve: forced.singleLegalAny },
  [PromptType.ChooseTargetSpell]: { showByDefault: true, resolve: forced.singleLegalSpell },

  [PromptType.RevealCards]: { showByDefault: true, resolve: informational.ackReveal },
  [PromptType.ChooseMode]: { showByDefault: true, resolve: forced.forcedAllModes },
  [PromptType.ChooseOptionalTrigger]: {
    showByDefault: true,
    resolve: triggerMemory.optionalTriggerMemory,
  },
  [PromptType.PayCostToPreventEffect]: { showByDefault: true },
  [PromptType.ChooseColor]: { showByDefault: true, resolve: forced.singleLegalColor },
  [PromptType.ChooseType]: { showByDefault: true, resolve: forced.singleLegalType },
  [PromptType.ChooseNumber]: { showByDefault: true, resolve: forced.singleLegalNumber },
  [PromptType.ChooseCardName]: { showByDefault: true, resolve: forced.singleLegalName },
  [PromptType.ChooseCardsForEffect]: { showByDefault: true, resolve: forced.allCardsForced },
  [PromptType.ChooseDiscard]: { showByDefault: true, resolve: forced.forcedDiscard },

  [PromptType.ChoosePhyrexian]: { showByDefault: true },
  [PromptType.ChooseKicker]: { showByDefault: true, resolve: optionalCosts.skipKicker },
  [PromptType.ChooseBuyback]: { showByDefault: true, resolve: optionalCosts.skipBuyback },
  [PromptType.ChooseMultikicker]: { showByDefault: true, resolve: optionalCosts.skipMultikicker },
  [PromptType.ChooseReplicate]: { showByDefault: true, resolve: optionalCosts.skipReplicate },
  [PromptType.ChooseAlternativeCost]: {
    showByDefault: true,
    resolve: forced.singleAlternativeCost,
  },
  [PromptType.PayManaCost]: { showByDefault: true },
  [PromptType.ChooseDelve]: { showByDefault: true, resolve: optionalCosts.skipDelve },
  [PromptType.ChooseConvoke]: { showByDefault: true, resolve: optionalCosts.skipConvoke },
  [PromptType.ChooseImprovise]: { showByDefault: true, resolve: optionalCosts.skipImprovise },
  [PromptType.SpecifyManaCombo]: { showByDefault: true },

  [PromptType.Scry]: { showByDefault: true, resolve: forced.emptyScry },
  [PromptType.Surveil]: { showByDefault: true, resolve: forced.emptySurveil },
  [PromptType.Dig]: { showByDefault: true, resolve: forced.emptyDig },
  [PromptType.ReorderLibrary]: { showByDefault: true, resolve: forced.singleCardOrder },

  [PromptType.ExploreDecision]: { showByDefault: true },
  [PromptType.HelpPayAssist]: { showByDefault: true, resolve: optionalCosts.skipAssist },

  [PromptType.FirstPlayerRoll]: { showByDefault: true, resolve: informational.ackFirstPlayerRoll },
  [PromptType.DiceRolled]: { showByDefault: true, resolve: informational.ackDiceRolled },
  [PromptType.ChooseRollToIgnore]: { showByDefault: true },
  [PromptType.ChooseRollToSwap]: { showByDefault: true },
  [PromptType.ChooseRollToModify]: { showByDefault: true },
  [PromptType.ChooseDiceToReroll]: { showByDefault: true },
  [PromptType.ChooseRollSwapValue]: { showByDefault: true },
};

export function resolvePrompt(prompt: AgentPrompt, ctx: ResolveCtx): AutoResolution {
  const handler = PROMPT_HANDLERS[prompt.type];
  if (!handler) return { kind: "force-show" };

  const resolverResult = handler.resolve?.(prompt, ctx) ?? { kind: "force-show" };
  if (resolverResult.kind === "auto") return resolverResult;

  const overridden = ctx.prefs.show[prompt.type];
  const show = overridden ?? handler.showByDefault;
  if (!show) {
    if (typeof console !== "undefined" && import.meta.env?.DEV) {
      console.warn(
        `[prompt-resolver] ${prompt.type} is toggled off but resolver returned force-show; ` +
          `showing modal as a fallback. Add a resolver branch if this case is auto-answerable.`,
      );
    }
  }
  return { kind: "force-show" };
}

export function effectiveShow(promptType: PromptType, prefs: PromptPreferencesSnapshot): boolean {
  return prefs.show[promptType] ?? PROMPT_HANDLERS[promptType].showByDefault;
}
