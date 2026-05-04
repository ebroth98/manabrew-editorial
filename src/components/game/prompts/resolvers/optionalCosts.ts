import { PromptType } from "@/types/promptType";
import { isToggledOff, type PromptResolver } from "../promptHandlers";

export const skipKicker: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.ChooseKicker, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "kickerDecision", kicked: false },
    reason: "kicker prompt toggled off; defaulting to skip",
  };
};

export const skipBuyback: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.ChooseBuyback, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "buybackDecision", buybackPaid: false },
    reason: "buyback prompt toggled off; defaulting to skip",
  };
};

export const skipMultikicker: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.ChooseMultikicker, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "multikickerDecision", kickCount: 0 },
    reason: "multikicker prompt toggled off; defaulting to 0",
  };
};

export const skipReplicate: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.ChooseReplicate, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "replicateDecision", replicateCount: 0 },
    reason: "replicate prompt toggled off; defaulting to 0",
  };
};

export const skipDelve: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.ChooseDelve, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "delveDecision", chosenCardIds: [] },
    reason: "delve prompt toggled off; defaulting to no delve",
  };
};

export const skipConvoke: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.ChooseConvoke, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "convokeDecision", chosenCardIds: [] },
    reason: "convoke prompt toggled off; defaulting to no convoke",
  };
};

export const skipImprovise: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.ChooseImprovise, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "improviseDecision", chosenCardIds: [] },
    reason: "improvise prompt toggled off; defaulting to no improvise",
  };
};

export const skipExertEnlist: PromptResolver = (prompt, ctx) => {
  const promptType =
    prompt.type === PromptType.ChooseExertAttackers
      ? PromptType.ChooseExertAttackers
      : PromptType.ChooseEnlistAttackers;
  if (!isToggledOff(promptType, ctx)) return { kind: "force-show" };
  const wireType =
    promptType === PromptType.ChooseExertAttackers ? "exertDecision" : "enlistDecision";
  return {
    kind: "auto",
    respond: { type: wireType, chosenAttackerIds: [] },
    reason: `${wireType} prompt toggled off; defaulting to none`,
  };
};

export const skipAssist: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.HelpPayAssist, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "assistDecision", amountToPay: 0 },
    reason: "help-pay prompt toggled off; defaulting to 0",
  };
};
