/**
 * Typed wrappers for Tauri backend commands.
 * Centralizes all invoke() calls for type safety and maintainability.
 * 
 * Note: These exports are the typed API layer for future migration.
 * They provide type-safe wrappers around Tauri invoke() calls.
 */
import { invoke } from "@tauri-apps/api/core";
import type { CardIdentity, GameFormat } from "@/types/server";

// ============================================================================
// Game Commands
// ============================================================================

export interface StartGameParams {
  deckList: { name: string; setCode: string }[];
  startingLife: number;
  commanderName: string | null;
  opponentDeckList: { name: string; setCode: string }[] | null;
}

export interface StartMultiplayerGameParams {
  playerNames: string[];
  deckLists: { name: string; setCode: string }[][];
  enginePlayerIndex: number;
  localIsHost: boolean;
  startingLife: number;
}

export interface RespondParams {
  action: Record<string, unknown>;
  playerSlot: string | null;
}

export interface RestoreSnapshotParams {
  checkpointId: number;
}

export const gameCommands = {
  /**
   * Start a single-player game with specified deck and configuration.
   * @returns Game session identifier string
   */
  startGame: (params: StartGameParams) =>
    invoke<string>("start_game", params as unknown as Record<string, unknown>),

  /**
   * Start a multiplayer game with multiple players and decks.
   */
  startMultiplayerGame: (params: StartMultiplayerGameParams) =>
    invoke<void>("start_multiplayer_game", params as unknown as Record<string, unknown>),

  /**
   * Send a player action/decision to the game engine.
   */
  respond: (params: RespondParams) =>
    invoke<void>("respond", params as unknown as Record<string, unknown>),

  /**
   * End the current game session.
   */
  endGame: () =>
    invoke<void>("end_game"),

  /**
   * Restore game state to a specific checkpoint.
   */
  restoreSnapshot: (params: RestoreSnapshotParams) =>
    invoke<void>("restore_snapshot", params as unknown as Record<string, unknown>),
};

// ============================================================================
// Server Commands
// ============================================================================

export interface ServerConnectParams {
  host: string;
  port: number;
  username: string;
  password: string;
}

export interface CreateRoomParams {
  roomName: string;
  maxPlayers: number;
  format: GameFormat;
}

export interface JoinRoomParams {
  roomId: string;
}

export interface SetReadyParams {
  ready: boolean;
}

export interface SetDeckSelectionParams {
  deckName: string;
  deckList: CardIdentity[];
  commanderName: string | null;
}

export const serverCommands = {
  /**
   * Connect to an XMage server.
   * Triggers 'server:auth_result' event on completion.
   */
  connect: (params: ServerConnectParams) =>
    invoke<void>("server_connect", params as unknown as Record<string, unknown>),

  /**
   * Disconnect from the current XMage server.
   */
  disconnect: () =>
    invoke<void>("server_disconnect"),

  /**
   * Request list of available game rooms.
   * Triggers 'server:room_list' event with results.
   */
  listRooms: () =>
    invoke<void>("server_list_rooms"),

  /**
   * Request list of connected players.
   * Triggers 'server:player_list' event with results.
   */
  listPlayers: () =>
    invoke<void>("server_list_players"),

  /**
   * Create a new game room.
   * Triggers 'server:room_created' event on success.
   */
  createRoom: (params: CreateRoomParams) =>
    invoke<void>("server_create_room", params as unknown as Record<string, unknown>),

  /**
   * Join an existing game room.
   * Triggers 'server:room_update' event with room state.
   */
  joinRoom: (params: JoinRoomParams) =>
    invoke<void>("server_join_room", params as unknown as Record<string, unknown>),

  /**
   * Leave the current game room.
   */
  leaveRoom: () =>
    invoke<void>("server_leave_room"),

  /**
   * Set ready status in current room.
   * Triggers 'server:ready_changed' event.
   */
  setReady: (params: SetReadyParams) =>
    invoke<void>("server_set_ready", params as unknown as Record<string, unknown>),

  /**
   * Set deck selection for the current room.
   */
  setDeckSelection: (params: SetDeckSelectionParams) =>
    invoke<void>("server_set_deck_selection", params as unknown as Record<string, unknown>),

  /**
   * Start the game in the current room (host only).
   * Triggers 'server:game_started' event on success.
   */
  startGame: () =>
    invoke<void>("server_start_game"),
};

// ============================================================================
// Deck & Utility Commands
// ============================================================================

export interface PresetDeckInfo {
  id: string;
  label: string;
  desc: string;
  color: string;
}

export const deckCommands = {
  /**
   * Get list of available preset decks.
   */
  getPresetDecks: () =>
    invoke<PresetDeckInfo[]>("get_preset_decks"),
};

export const debugCommands = {
  /**
   * Get current game prompt (debug utility).
   */
  getPrompt: () =>
    invoke<unknown>("get_prompt"),
};

// ============================================================================
// Consolidated API Export
// ============================================================================

/**
 * Centralized Tauri command API.
 * Use this instead of direct invoke() calls for type safety and discoverability.
 */
export const tauriApi = {
  game: gameCommands,
  server: serverCommands,
  deck: deckCommands,
  debug: debugCommands,
};
