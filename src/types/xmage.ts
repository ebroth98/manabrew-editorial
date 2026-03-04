// Core XMage Types (mirrored from Java)

export interface Card {
  id: string; // UUID
  name: string;
  setCode: string;
  cardNumber: string;
  color: string;
  manaCost: string;
  cmc?: number;
  types: string[];
  subtypes: string[];
  supertypes: string[];
  power?: string;
  toughness?: string;
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
}

export interface Deck {
  name: string;
  cards: Card[];
  sideboard: Card[];
  /** Designated commander (Commander format). Not included in cards[]. */
  commander?: Card;
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
  name: string;
  text: string;
}

export interface ActivatableAbilityInfo {
  cardId: string;
  abilityIndex: number;
  description: string;
  isManaAbility: boolean;
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
