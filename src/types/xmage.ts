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
}

export interface Deck {
  name: string;
  cards: Card[];
  sideboard: Card[];
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
  activePlayerId: string; // UUID
  priorityPlayerId: string; // UUID
  players: Player[];
  myHand: Card[];
  battlefield: Card[]; // Simplified for now, likely zoned
  stack: StackObject[];
  exile: Card[];
  graveyard: Card[];
}

export interface StackObject {
  id: string; // UUID
  sourceId: string; // UUID
  name: string;
  text: string;
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
