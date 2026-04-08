// Core OpenMagic Types (mirrored from Java forge-game)

export type DeckFormatId =
  | "standard"
  | "pioneer"
  | "modern"
  | "legacy"
  | "vintage"
  | "pauper"
  | "commander"
  | "brawl"
  | "oathbreaker"
  | "draft"
  | "sealed";

export interface Card {
  id: string; // UUID
  name: string;
  setCode: string;
  cardNumber: string;
  color: string;
  colorIdentity?: string[];
  manaCost: string;
  cmc?: number;
  types: string[];
  subtypes: string[];
  supertypes: string[];
  power?: string;
  toughness?: string;
  /** Base power before modifiers (for buff/debuff color-coding). */
  basePower?: number;
  /** Base toughness before modifiers (for buff/debuff color-coding). */
  baseToughness?: number;
  text: string;
  imageUrl?: string;
  isPlayable: boolean;
  isSelected: boolean;
  isChoosable: boolean;
  controllerId: string; // UUID
  ownerId: string; // UUID
  zoneId: string; // UUID
  tapped?: boolean;
  keywords?: string[];
  /** Active counters keyed by counter type name (e.g. "P1P1", "M1M1", "Loyalty"). Only non-zero entries present. */
  counters?: Record<string, number>;
  damage?: number;
  summoningSick?: boolean;
  isToken?: boolean;
  /** True if this card has an alternate face (Transform DFC, Modal DFC). */
  isDoubleFaced?: boolean;
  /** True if this card is currently showing its back face. */
  isTransformed?: boolean;
  /** True if this card is face-down (Morph, Manifest). */
  isFaceDown?: boolean;
  /** True if this card is currently bestowed (attached as an Aura via Bestow). */
  isBestowed?: boolean;
  /** ID of the card this permanent is attached to (equipment host, enchanted creature). */
  attachedTo?: string;
  /** IDs of cards attached to this permanent (equipment, auras). */
  attachmentIds?: string[];
  /** True if this card is phased out (treated as not on battlefield). */
  phasedOut?: boolean;
  /** True if this creature has been exerted (won't untap next untap step). */
  exerted?: boolean;
  /** Flashback cost string if the card has flashback (e.g. "1 R"). */
  flashbackCost?: string;
  /** Kicker cost string if the card has kicker (e.g. "W"). */
  kickerCost?: string;
  /** Effective mana cost after static ability reductions/increases. Only set when different from manaCost. */
  effectiveManaCost?: string;
  /** Madness cost string if the card has madness (e.g. "R"). */
  madnessCost?: string;
  /** True if this card is currently exiled via Madness replacement. */
  isMadnessExiled?: boolean;
  /** True if this card has been plotted (castable from exile for free on a later turn). */
  isPlotted?: boolean;
  /** True if this card was exiled via Warp (castable from exile for normal cost). */
  isWarpExiled?: boolean;
}

export interface DeckLabel {
  name: string;
  color?: string;
}

export interface Deck {
  name: string;
  format?: DeckFormatId;
  cards: Card[];
  sideboard: Card[];
  /** Supplementary Attraction deck, separate from the sideboard like Forge RegisteredPlayer.getAttractions(). */
  attractions?: Card[];
  /** Supplementary Contraption deck, separate from the sideboard like Forge RegisteredPlayer.getContraptions(). */
  contraptions?: Card[];
  /** Supplementary Scheme deck, separate from the sideboard like Forge RegisteredPlayer.getSchemes(). */
  schemes?: Card[];
  /** Supplementary Planar deck, separate from the sideboard like Forge RegisteredPlayer.getPlanes(). */
  planes?: Card[];
  /** Designated commander(s) (Commander format). Not included in cards[]. Supports Partner. */
  commanders?: Card[];
  /** Designated companion (any format). Not included in cards[] or sideboard[]. */
  companion?: Card;
  /** Cards being considered but not in the playable deck. */
  maybeboard?: Card[];
  /** When true, deck is a work-in-progress and not playable. */
  draft?: boolean;
  /** User-assigned labels for the deck (e.g. "Aggro", "Budget", "Competitive"). */
  labels?: DeckLabel[];
  /** User-created tag/label names for organizing cards into custom sections. */
  customTags?: string[];
  /** Maps card name (lowercased) → array of tag names the card belongs to. */
  cardTags?: Record<string, string[]>;
  /** Name of the card whose art is used as the deck cover. Falls back to cards[0] when absent or card no longer in deck. */
  coverCardName?: string;
  /** Which face of a double-faced cover card to use: 0 = front (default), 1 = back. */
  coverCardFace?: 0 | 1;
  /** Saved stack-view section positions (section ID → {x, y} in pixels). */
  stackPositions?: Record<string, { x: number; y: number }>;
}

export interface Player {
  id: string; // UUID
  name: string;
  isHuman: boolean;
  life: number;
  poison: number;
  handCount: number;
  libraryCount: number;
  graveyardCount: number;
  exileCount: number;
  manaPool: Record<string, number>; // W, U, B, R, G, C
  /** Commander damage received: source card id → total damage. */
  commanderDamage?: Record<string, number>;
  /** Energy counters (Kaladesh block). */
  energyCounters?: number;
}

export interface Table {
  id: string; // UUID
  name: string;
  gameType: string;
  deckType: string;
  state: 'WAITING' | 'DUELING' | 'SIDEBOARDING' | 'FINISHED';
  numPlayers: number;
  players: PlayerInfo[];
  isTournament: boolean;
}

export interface PlayerInfo {
  name: string;
  avatar: string; // ID or path
  flag: string; // Country code
}

export interface GameView {
  gameId: string; // UUID
  turn: number;
  step: string; // Phase/Step name
  combatAssignments?: CombatAssignment[];
  activePlayerId: string; // UUID
  priorityPlayerId: string; // UUID
  players: Player[];
  myHand: Card[];
  battlefield: Card[]; // Simplified for now, likely zoned
  stack: StackObject[];
  exile: Card[];
  graveyard: Card[];
  opponentGraveyard: Card[];
  opponentExile: Card[];
  /** Cards in the human player's command zone (typically just the commander). */
  myCommandZone?: Card[];
  /** Cards in the opponent's command zone. */
  opponentCommandZone?: Card[];
  gameOver?: boolean;
  winnerId?: string | null;
  /** The player who is the current monarch. */
  monarchId?: string | null;
  /** The player who holds the initiative. */
  initiativeHolderId?: string | null;
}

export interface CombatAssignment {
  blockerId: string;
  attackerId: string;
}

export interface StackObject {
  id: string; // UUID
  sourceId: string; // UUID
  /** Player who cast/activated this spell or ability. */
  controllerId: string;
  name: string;
  text: string;
  /** True for permanent spells (creature/artifact/enchantment/planeswalker). */
  isPermanentSpell: boolean;
  /** Normalized chosen targets flattened across ability/sub-ability nodes. */
  targets: StackTarget[];
}

export type StackTargetKind = "card" | "player" | "stack";

export interface StackTarget {
  kind: StackTargetKind;
  /** Encoded game entity id: card-*, player-*, stack-* */
  id: string;
  /** Zero-based index in the ability chain (root node = 0). */
  nodeIndex: number;
  /** Zero-based target slot index within the node. */
  targetIndex: number;
  /** Whether this target is hostile (damage/destroy) vs friendly (buff). */
  hostile: boolean;
}

export interface ActivatableAbilityInfo {
  cardId: string;
  abilityIndex: number;
  description: string;
  isManaAbility: boolean;
  cost?: string;
}

export interface ClientCallback {
  id: string; // UUID
  method: string; // e.g., "askYesNo", "chooseMode"
  data: any; // Context specific payload
}

// Middleware API Response Types
export interface MiddlewareResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface LoginResponse {
  sessionId: string;
  user: User;
}

export interface User {
  username: string;
  serverAddress: string;
  flag?: string;
}
