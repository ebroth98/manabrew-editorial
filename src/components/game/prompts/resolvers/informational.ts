import { PromptType } from "@/types/promptType";
import { isToggledOff, type PromptResolver } from "../promptHandlers";

export const ackReveal: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.RevealCards, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "revealCardsAcknowledged" },
    reason: "RevealCards toggled off; auto-ack",
  };
};

export const ackDiceRolled: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.DiceRolled, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "diceRolledAcknowledged" },
    reason: "DiceRolled toggled off; auto-ack",
  };
};

export const ackFirstPlayerRoll: PromptResolver = (_prompt, ctx) => {
  if (!isToggledOff(PromptType.FirstPlayerRoll, ctx)) return { kind: "force-show" };
  return {
    kind: "auto",
    respond: { type: "firstPlayerRollAcknowledged" },
    reason: "FirstPlayerRoll toggled off; auto-ack",
  };
};
