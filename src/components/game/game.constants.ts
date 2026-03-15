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

export const PROMPT_LABELS: Record<string, string> = {
  mulligan: "Keep this hand?",
  mulliganPutBack: "Choose cards to put on bottom",
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
  payCombatCost: "Pay attack cost",
  payManaCost: "Pay mana cost",
  chooseColor: "Choose a color",
  chooseType: "Choose a type",
  chooseNumber: "Choose a number",
  chooseCardName: "Choose a card name",
  chooseDelve: "Choose cards to exile for Delve",
  chooseConvoke: "Choose creatures to tap for Convoke",
  chooseImprovise: "Choose artifacts to tap for Improvise",
  specifyManaCombo: "Choose mana colors",
  chooseDamageAssignmentOrder: "Order blockers for damage assignment",
  chooseCardsForEffect: "Choose cards for effect",
  choosePhyrexian: "Pay Phyrexian mana with life?",
  chooseExertAttackers: "Choose attackers to exert",
  chooseEnlistAttackers: "Choose attackers to enlist",
  reorderLibrary: "Reorder the top of your library",
  exploreDecision: "Explore: put in graveyard or on top?",
  helpPayAssist: "Help pay for a spell?",
  gameOver: "Game Over",
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
