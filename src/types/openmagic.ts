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
  basePower?: number;
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
  counters?: Record<string, number>;
  damage?: number;
  summoningSick?: boolean;
  isToken?: boolean;
  isDoubleFaced?: boolean;
  isTransformed?: boolean;
  isFaceDown?: boolean;
  isBestowed?: boolean;
  attachedTo?: string;
  attachmentIds?: string[];
  phasedOut?: boolean;
  exerted?: boolean;
  flashbackCost?: string;
  kickerCost?: string;
  effectiveManaCost?: string;
  madnessCost?: string;
  isMadnessExiled?: boolean;
  isPlotted?: boolean;
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
  /** Cached token metadata for cards in this deck (persisted to avoid re-fetching). */
  tokens?: DeckToken[];
}

/** Token metadata produced by a card in the deck. */
export interface DeckToken {
  /** Token name as it appears on Scryfall. */
  name: string;
  /** Type line, e.g. "Token Creature — Angel". */
  typeLine: string;
  /** Names of deck cards that produce this token. */
  producers: string[];
  /** User-selected printing set code (e.g. "thou"). */
  setCode?: string;
  /** Collector number within the set. */
  cardNumber?: string;
  /** Resolved image URL for the selected printing. */
  imageUrl?: string;
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
  /** Radiation counters (Fallout Commander). */
  radiationCounters?: number;
  /** City's Blessing status (Ascend). */
  hasCityBlessing?: boolean;
  /** The Ring tempts you: 0 = no ring, 1-4 = level of temptation. */
  ringLevel?: number;
  /** Start Your Engines speed (Aetherdrift): 0 = no speed, 1-4. */
  speed?: number;
}

export interface Table {
  id: string; // UUID
  name: string;
  gameType: string;
  deckType: string;
  state: "WAITING" | "DUELING" | "SIDEBOARDING" | "FINISHED";
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
  /** Whether this target is hostile (damage/destroy) vs friendly (buff).
   *  Kept for backwards compatibility; prefer `intent`. */
  hostile: boolean;
  /** Semantic classification used to pick a pointer icon and glow color. */
  intent: import("@/types/promptType").TargetingIntent;
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
