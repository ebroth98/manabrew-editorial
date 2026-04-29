export type GameFormat =
  | "Standard"
  | "Pioneer"
  | "Modern"
  | "Legacy"
  | "Vintage"
  | "Pauper"
  | "Commander"
  | "Brawl"
  | "Oathbreaker"
  | "Draft"
  | "Sealed";
export type DeckSection =
  | "main"
  | "sideboard"
  | "commander"
  | "attractions"
  | "contraptions"
  | "schemes"
  | "planes";

export interface CardIdentity {
  name: string;
  setCode: string;
  section?: DeckSection;
  foil?: boolean;
}

export interface RoomInfo {
  room_id: string;
  room_name: string;
  host: string;
  hosted: boolean;
  players: RoomPlayerInfo[];
  max_players: number;
  format: GameFormat;
  status: "Lobby" | "InGame";
}

export interface RoomPlayerInfo {
  username: string;
  ready: boolean;
  connected: boolean;
  selected_deck_name?: string;
}

export interface PlayerDeckInfo {
  username: string;
  deck_name: string;
  deck_list: CardIdentity[];
  commander_name?: string;
}

export interface PlayerInfo {
  username: string;
  player_id: string;
  connected: boolean;
  room_id?: string;
}

export interface AuthResultPayload {
  success: boolean;
  player_id: string | null;
  reconnected: boolean | null;
  error: string | null;
}

export interface RoomListPayload {
  rooms: RoomInfo[];
}

export interface PlayerListPayload {
  players: PlayerInfo[];
}

export interface RoomCreatedPayload {
  room_id: string;
  room_name: string;
}

export interface RoomUpdatePayload {
  room: RoomInfo;
}

export interface PlayerJoinedPayload {
  room_id: string;
  username: string;
}

export interface PlayerLeftPayload {
  room_id: string;
  username: string;
}

export interface PlayerConnectionPayload {
  username: string;
}

export interface ReadyChangedPayload {
  username: string;
  ready: boolean;
}

export interface GameStartedPayload {
  room_id: string;
  player_order: string[];
  player_decks: PlayerDeckInfo[];
  starting_life: number;
}

export interface StateUpdatePayload {
  from_player: string;
  state: unknown;
}

export const ROOM_RELAY_KIND = "roomRelay" as const;

export interface RoomRelayEnvelope<TPayload = unknown> {
  kind: typeof ROOM_RELAY_KIND;
  protocol: string;
  version: 1;
  messageId: string;
  fromPlayer?: string;
  targetPlayer?: string;
  roomId?: string;
  payload: TPayload;
}

export interface RoomMessagePayload<TPayload = unknown> {
  from_player: string;
  state: RoomRelayEnvelope<TPayload>;
}

export function isRoomRelayEnvelope(value: unknown): value is RoomRelayEnvelope {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<RoomRelayEnvelope>;
  return (
    candidate.kind === ROOM_RELAY_KIND &&
    typeof candidate.protocol === "string" &&
    candidate.version === 1 &&
    typeof candidate.messageId === "string" &&
    "payload" in candidate
  );
}

export interface TurnChangedPayload {
  from_player: string;
  new_active_player: string;
  turn_number: number;
}

export interface ServerErrorPayload {
  code: string;
  message: string;
}
