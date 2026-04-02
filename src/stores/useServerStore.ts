import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import { tauriApi } from '@/api/tauri';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  RoomInfo,
  PlayerInfo,
  GameFormat,
  CardIdentity,
  PlayerDeckInfo,
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
  playerDecks: PlayerDeckInfo[];
  startingLife: number;

  connect(host: string, port: number, username: string, password: string): Promise<void>;
  disconnect(): Promise<void>;
  listRooms(): Promise<void>;
  listPlayers(): Promise<void>;
  createRoom(roomName: string, maxPlayers: number, format: GameFormat): Promise<void>;
  joinRoom(roomId: string): Promise<void>;
  leaveRoom(): Promise<void>;
  setReady(ready: boolean): Promise<void>;
  setDeckSelection(deckName: string, deckList: CardIdentity[], commanderName?: string): Promise<void>;
  startGame(): Promise<void>;

  setupListeners(): () => void;
}

export const useServerStore = create<ServerState>()(devtools((set, get) => ({
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
  playerDecks: [],
  startingLife: 20,

  async connect(host, port, username, password) {
    set({ username, connecting: true, error: null });
    try {
      await tauriApi.server.connect({ host, port, username, password });
    } catch (e) {
      set({ connecting: false, error: String(e) });
    }
  },

  async disconnect() {
    await tauriApi.server.disconnect();
    set({
      connected: false,
      playerId: null,
      username: null,
      currentRoom: null,
      gameStarted: false,
      playerOrder: [],
      playerDecks: [],
      startingLife: 20,
      rooms: [],
      players: [],
    });
  },

  async listRooms() {
    await tauriApi.server.listRooms();
  },

  async listPlayers() {
    await tauriApi.server.listPlayers();
  },

  async createRoom(roomName, maxPlayers, format) {
    await tauriApi.server.createRoom({ roomName, maxPlayers, format });
  },

  async joinRoom(roomId) {
    await tauriApi.server.joinRoom({ roomId });
  },

  async leaveRoom() {
    await tauriApi.server.leaveRoom();
    set({ currentRoom: null });
    get().listRooms();
  },

  async setReady(ready) {
    await tauriApi.server.setReady({ ready });
  },

  async setDeckSelection(deckName, deckList, commanderName) {
    await tauriApi.server.setDeckSelection({ deckName, deckList, commanderName: commanderName ?? null });
  },

  async startGame() {
    await tauriApi.server.startGame();
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
            playerDecks: e.payload.player_decks,
            startingLife: e.payload.starting_life,
          });
        }),
      );

      unlisteners.push(
        await listen<ServerErrorPayload>('server:error', (e) => {
          console.error('[server] error:', e.payload.code, e.payload.message);
          if (e.payload.code === 'not_in_room') {
            set({
              currentRoom: null,
              gameStarted: false,
              playerOrder: [],
              playerDecks: [],
              startingLife: 20,
            });
            void get().listRooms();
          }
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
            playerDecks: [],
            startingLife: 20,
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
}), { name: "server", enabled: import.meta.env.DEV }));
