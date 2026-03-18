/** Phase bar definitions — must match phase_to_step() in src-tauri/src/game_view_dto.rs */
export const PHASES = [
  { id: "untap", label: "Untap", short: "UNT" },
  { id: "upkeep", label: "Upkeep", short: "UP" },
  { id: "draw", label: "Draw", short: "DR" },
  { id: "main1", label: "Main 1", short: "M1" },
  { id: "begin_combat", label: "Begin Combat", short: "BC" },
  { id: "declare_attackers", label: "Attackers", short: "ATK" },
  { id: "declare_blockers", label: "Blockers", short: "BLK" },
  { id: "first_strike_damage", label: "1st Strike", short: "1ST" },
  { id: "combat_damage", label: "Damage", short: "DMG" },
  { id: "end_combat", label: "End Combat", short: "EC" },
  { id: "main2", label: "Main 2", short: "M2" },
  { id: "end", label: "End", short: "END" },
  { id: "cleanup", label: "Cleanup", short: "CL" },
] as const;

export const MANA_COLORS = [
  { key: "W", bg: "bg-yellow-50 border-yellow-200", text: "text-yellow-800" },
  { key: "U", bg: "bg-blue-100 border-blue-300", text: "text-blue-800" },
  { key: "B", bg: "bg-gray-800 border-gray-600", text: "text-gray-100" },
  { key: "R", bg: "bg-red-100 border-red-300", text: "text-red-800" },
  { key: "G", bg: "bg-green-100 border-green-300", text: "text-green-800" },
  { key: "C", bg: "bg-gray-100 border-gray-300", text: "text-gray-700" },
] as const;

export const AVATAR_COLORS = [
  "bg-blue-600 text-white",
  "bg-purple-600 text-white",
  "bg-red-600 text-white",
  "bg-green-700 text-white",
  "bg-orange-500 text-white",
  "bg-pink-600 text-white",
  "bg-teal-600 text-white",
  "bg-indigo-600 text-white",
] as const;

export const ZONE_COLUMN_RESERVED_PX = 68;

import { PromptType } from "@/types/promptType";

export const PROMPT_LABELS: Record<string, string> = {
  [PromptType.Mulligan]: "Keep this hand?",
  [PromptType.MulliganPutBack]: "Choose cards to put on bottom",
  [PromptType.ChooseAction]: "Play a card or pass priority",
  [PromptType.ChooseAttackers]: "Declare attackers",
  [PromptType.ChooseBlockers]: "Declare blockers",
  [PromptType.ChooseTargetPlayer]: "Choose a target player",
  [PromptType.ChooseTargetCard]: "Choose a target creature",
  [PromptType.ChooseTargetAny]: "Choose a target (player or permanent)",
  [PromptType.ChooseTargetCardFromZone]: "Choose a target card from the zone",
  [PromptType.ChooseTargetSpell]: "Choose a spell on the stack to counter",
  [PromptType.ChooseMode]: "Choose a mode for the spell",
  [PromptType.ChooseOptionalTrigger]: "An optional ability would trigger",
  [PromptType.ChooseKicker]: "Pay the kicker cost?",
  [PromptType.ChooseBuyback]: "Pay the buyback cost?",
  [PromptType.ChooseMultikicker]: "Choose multikicker count",
  [PromptType.ChooseReplicate]: "Choose replicate count",
  [PromptType.ChooseAlternativeCost]: "Choose casting option",
  [PromptType.Scry]: "Scry: choose cards to put on the bottom",
  [PromptType.Surveil]: "Surveil: choose cards to send to graveyard",
  [PromptType.Dig]: "Dig: choose cards to take",
  [PromptType.ChooseDiscard]: "Discard cards",
  [PromptType.PayCombatCost]: "Pay attack cost",
  [PromptType.PayManaCost]: "Pay mana cost",
  [PromptType.ChooseColor]: "Choose a color",
  [PromptType.ChooseType]: "Choose a type",
  [PromptType.ChooseNumber]: "Choose a number",
  [PromptType.ChooseCardName]: "Choose a card name",
  [PromptType.ChooseDelve]: "Choose cards to exile for Delve",
  [PromptType.ChooseConvoke]: "Choose creatures to tap for Convoke",
  [PromptType.ChooseImprovise]: "Choose artifacts to tap for Improvise",
  [PromptType.SpecifyManaCombo]: "Choose mana colors",
  [PromptType.ChooseDamageAssignmentOrder]: "Order blockers for damage assignment",
  [PromptType.ChooseCardsForEffect]: "Choose cards for effect",
  [PromptType.ChoosePhyrexian]: "Pay Phyrexian mana with life?",
  [PromptType.ChooseExertAttackers]: "Choose attackers to exert",
  [PromptType.ChooseEnlistAttackers]: "Choose attackers to enlist",
  [PromptType.ReorderLibrary]: "Reorder the top of your library",
  [PromptType.ExploreDecision]: "Explore: put in graveyard or on top?",
  [PromptType.HelpPayAssist]: "Help pay for a spell?",
  [PromptType.GameOver]: "Game Over",
};

/** Card status badge definitions — label + Tailwind color classes */
export const CARD_BADGES = {
  exerted:     { label: "EXERTED",     style: "bg-orange-500/90 text-white" },
  morph:       { label: "MORPH",       style: "bg-gray-600/90 text-white" },
  bestow:      { label: "BESTOW",      style: "bg-teal-500/90 text-white" },
  token:       { label: "TOKEN",       style: "bg-amber-400/90 text-amber-900" },
  transformed: { label: "TRANSFORMED", style: "bg-purple-500/90 text-white" },
} as const;

/** Logical footprint of battlefield cards (for FreeBattlefield grid layout) */
export const CARD_W = 72;
export const CARD_H = 100;
export const CARD_GAP = 8;
