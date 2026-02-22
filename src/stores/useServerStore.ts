import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  RoomInfo,
  PlayerInfo,
  GameFormat,
  AuthResultPayload,
  RoomListPayload,
  PlayerListPayload,
  RoomUpdatePayload,
  RoomCreatedPayload,
  PlayerJoinedPayload,
  PlayerLeftPayload,
  PlayerConnectionPayload,
  ReadyChangedPayload,
  GameStartedPayload,
  ServerErrorPayload,
} from '@/types/server';

interface ServerState {
  connected: boolean;
  connecting: boolean;
  error: string | null;
  playerId: string | null;
  username: string | null;

  rooms: RoomInfo[];
  currentRoom: RoomInfo | null;
  players: PlayerInfo[];

  gameStarted: boolean;
  playerOrder: string[];

  connect(host: string, port: number, username: string, password: string): Promise<void>;
  disconnect(): Promise<void>;
  listRooms(): Promise<void>;
  listPlayers(): Promise<void>;
  createRoom(roomName: string, maxPlayers: number, format: GameFormat): Promise<void>;
  joinRoom(roomId: string): Promise<void>;
  leaveRoom(): Promise<void>;
  setReady(ready: boolean): Promise<void>;
  startGame(): Promise<void>;

  setupListeners(): () => void;
}

export const useServerStore = create<ServerState>()((set, get) => ({
  connected: false,
  connecting: false,
  error: null,
  playerId: null,
  username: null,
  rooms: [],
  currentRoom: null,
  players: [],
  gameStarted: false,
  playerOrder: [],

  async connect(host, port, username, password) {
    set({ username, connecting: true, error: null });
    try {
      await invoke('server_connect', { host, port, username, password });
    } catch (e) {
      set({ connecting: false, error: String(e) });
    }
  },

  async disconnect() {
    await invoke('server_disconnect');
    set({
      connected: false,
      playerId: null,
      username: null,
      currentRoom: null,
      gameStarted: false,
      playerOrder: [],
      rooms: [],
      players: [],
    });
  },

  async listRooms() {
    await invoke('server_list_rooms');
  },

  async listPlayers() {
    await invoke('server_list_players');
  },

  async createRoom(roomName, maxPlayers, format) {
    await invoke('server_create_room', { roomName, maxPlayers, format });
  },

  async joinRoom(roomId) {
    await invoke('server_join_room', { roomId });
  },

  async leaveRoom() {
    await invoke('server_leave_room');
    set({ currentRoom: null });
    get().listRooms();
  },

  async setReady(ready) {
    await invoke('server_set_ready', { ready });
  },

  async startGame() {
    await invoke('server_start_game');
  },

  setupListeners() {
    const unlisteners: UnlistenFn[] = [];

    const setup = async () => {
      unlisteners.push(
        await listen<AuthResultPayload>('server:auth_result', (e) => {
          if (e.payload.success) {
            set({ connected: true, connecting: false, error: null, playerId: e.payload.player_id });
            get().listRooms();
            get().listPlayers();
          } else {
            set({ connecting: false, error: e.payload.error ?? 'Authentication failed' });
          }
        }),
      );

      unlisteners.push(
        await listen<RoomListPayload>('server:room_list', (e) => {
          set({ rooms: e.payload.rooms });
        }),
      );

      unlisteners.push(
        await listen<PlayerListPayload>('server:player_list', (e) => {
          set({ players: e.payload.players });
        }),
      );

      unlisteners.push(
        await listen<RoomCreatedPayload>('server:room_created', () => {
          get().listRooms();
        }),
      );

      unlisteners.push(
        await listen<RoomUpdatePayload>('server:room_update', (e) => {
          set({ currentRoom: e.payload.room });
        }),
      );

      unlisteners.push(
        await listen<PlayerJoinedPayload>('server:player_joined', () => {
          get().listRooms();
          get().listPlayers();
        }),
      );

      unlisteners.push(
        await listen<PlayerLeftPayload>('server:player_left', () => {
          get().listRooms();
          get().listPlayers();
        }),
      );

      unlisteners.push(
        await listen<PlayerConnectionPayload>('server:player_connected', () => {
          get().listPlayers();
        }),
      );

      unlisteners.push(
        await listen<PlayerConnectionPayload>('server:player_disconnected', () => {
          get().listPlayers();
        }),
      );

      unlisteners.push(
        await listen<ReadyChangedPayload>('server:ready_changed', () => {
          // Room update will come separately with full state
        }),
      );

      unlisteners.push(
        await listen<GameStartedPayload>('server:game_started', (e) => {
          set({
            gameStarted: true,
            playerOrder: e.payload.player_order,
          });
        }),
      );

      unlisteners.push(
        await listen<ServerErrorPayload>('server:error', () => {
          // Errors handled by individual callers or shown in UI
        }),
      );

      unlisteners.push(
        await listen('server:disconnected', () => {
          set({
            connected: false,
            connecting: false,
            error: 'Disconnected from server',
            playerId: null,
            currentRoom: null,
            gameStarted: false,
            playerOrder: [],
            rooms: [],
            players: [],
          });
        }),
      );

      unlisteners.push(
        await listen('server:state_update', () => {}),
      );

      unlisteners.push(
        await listen('server:turn_changed', () => {}),
      );
    };

    setup();

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  },
}));
