import type { PromptResolver } from "../promptHandlers";

export const ackReveal: PromptResolver = () => ({
  kind: "auto",
  respond: { type: "revealCardsAcknowledged" },
  reason: "RevealCards is informational; auto-ack",
});

export const ackDiceRolled: PromptResolver = () => ({
  kind: "auto",
  respond: { type: "diceRolledAcknowledged" },
  reason: "DiceRolled is informational; auto-ack",
});

export const ackFirstPlayerRoll: PromptResolver = () => ({
  kind: "auto",
  respond: { type: "firstPlayerRollAcknowledged" },
  reason: "FirstPlayerRoll is informational; auto-ack",
});
