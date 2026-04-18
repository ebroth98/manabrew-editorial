/**
 * All prompt types that the Rust engine can send to the UI.
 * Used as the `type` field in `AgentPrompt`.
 *
 * Using a const enum ensures zero runtime overhead — values are
 * inlined as string literals at compile time.
 */
export const PromptType = {
  // Game lifecycle
  StateUpdate: "stateUpdate",
  GameOver: "gameOver",

  // Mulligan
  Mulligan: "mulligan",
  MulliganPutBack: "mulliganPutBack",

  // Main phase actions
  ChooseAction: "chooseAction",

  // Combat
  ChooseAttackers: "chooseAttackers",
  ChooseBlockers: "chooseBlockers",
  ChooseExertAttackers: "chooseExertAttackers",
  ChooseEnlistAttackers: "chooseEnlistAttackers",
  ChooseDamageAssignmentOrder: "chooseDamageAssignmentOrder",
  ChooseCombatDamageAssignment: "chooseCombatDamageAssignment",
  PayCombatCost: "payCombatCost",

  // Targeting
  ChooseTargetCard: "chooseTargetCard",
  ChooseTargetCardFromZone: "chooseTargetCardFromZone",
  ChooseTargetPlayer: "chooseTargetPlayer",
  ChooseTargetAny: "chooseTargetAny",
  ChooseTargetSpell: "chooseTargetSpell",

  // Modal choices
  RevealCards: "revealCards",
  ChooseMode: "chooseMode",
  ChooseOptionalTrigger: "chooseOptionalTrigger",
  PayCostToPreventEffect: "payCostToPreventEffect",
  ChooseColor: "chooseColor",
  ChooseType: "chooseType",
  ChooseNumber: "chooseNumber",
  ChooseCardName: "chooseCardName",
  ChooseCardsForEffect: "chooseCardsForEffect",
  ChooseDiscard: "chooseDiscard",

  // Cost payment
  ChoosePhyrexian: "choosePhyrexian",
  ChooseKicker: "chooseKicker",
  ChooseBuyback: "chooseBuyback",
  ChooseMultikicker: "chooseMultikicker",
  ChooseReplicate: "chooseReplicate",
  ChooseAlternativeCost: "chooseAlternativeCost",
  PayManaCost: "payManaCost",
  ChooseDelve: "chooseDelve",
  ChooseConvoke: "chooseConvoke",
  ChooseImprovise: "chooseImprovise",
  SpecifyManaCombo: "specifyManaCombo",

  // Library manipulation
  Scry: "scry",
  Surveil: "surveil",
  Dig: "dig",
  ReorderLibrary: "reorderLibrary",

  // Other decisions
  ExploreDecision: "exploreDecision",
  HelpPayAssist: "helpPayAssist",
} as const;

export type PromptType = (typeof PromptType)[keyof typeof PromptType];
