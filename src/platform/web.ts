/**
 * Web platform implementation.
 *
 * This provides the backend for pure web deployments using WASM.
 * The game engine runs in a Web Worker for non-blocking UI.
 */

import type {
  IPlatformApi,
  IGameApi,
  IServerApi,
  IStorageApi,
  IEventBus,
  PlatformFeature,
  StartGameParams,
  StartMultiplayerGameParams,
  RespondParams,
  RestoreSnapshotParams,
  PresetDeckInfo,
  DeckAvailabilityResult,
  ServerConnectParams,
  CreateRoomParams,
  JoinRoomParams,
  SetReadyParams,
  SetDeckSelectionParams,
  SpawnAiBotParams,
} from "./types";
import { isRoomRelayEnvelope } from "@/types/server";
import type { RoomRelayEnvelope } from "@/types/server";

// ============================================================================
// Worker Message Types
// ============================================================================

interface WorkerCommand {
  type: "command";
  requestId: string;
  command: string;
  args?: Record<string, unknown>;
}

interface WorkerResponse {
  type: "response";
  requestId: string;
  payload?: unknown;
  error?: string;
}

interface WorkerEvent {
  type: "event";
  event: string;
  payload: unknown;
}

type WorkerMessage = WorkerResponse | WorkerEvent;

// ============================================================================
// Worker Bridge
// ============================================================================

/**
 * Bridge for communicating with the game engine worker.
 */
class WorkerBridge {
  private worker: Worker | null = null;
  private pendingRequests = new Map<
    string,
    { resolve: (value: unknown) => void; reject: (error: Error) => void }
  >();
  private eventBus: WebEventBus;
  private initPromise: Promise<void> | null = null;

  /** SharedArrayBuffer for local player prompt/response */
  gameBuffer: SharedArrayBuffer | null = null;
  private gameSignal: Int32Array | null = null;
  private gameData: Uint8Array | null = null;

  /** SharedArrayBuffer for remote player relay (multiplayer hosting) */
  private remoteBuffer: SharedArrayBuffer | null = null;
  private remoteSignal: Int32Array | null = null;
  private remoteData: Uint8Array | null = null;

  constructor(eventBus: WebEventBus) {
    this.eventBus = eventBus;

    // Listen for SAB from worker and start polling for prompts
    eventBus.on<{ buffer: SharedArrayBuffer }>("game:sab", (payload) => {
      this.gameBuffer = payload.buffer;
      this.gameSignal = new Int32Array(this.gameBuffer, 0, 2);
      this.gameData = new Uint8Array(this.gameBuffer, 8);
      console.log("[WorkerBridge] Received local SAB, starting prompt poll");
      this.pollForPrompts();
    });

    // Listen for remote player SAB (multiplayer hosting)
    eventBus.on<{ buffer: SharedArrayBuffer }>("game:remote_sab", (payload) => {
      this.remoteBuffer = payload.buffer;
      this.remoteSignal = new Int32Array(this.remoteBuffer, 0, 2);
      this.remoteData = new Uint8Array(this.remoteBuffer, 8);
      console.log("[WorkerBridge] Received remote SAB, starting relay poll");
      this.pollForRemotePrompts();
    });
  }

  /**
   * Poll the SAB for prompts from the game engine (runs on main thread).
   * When the engine writes a prompt (signal=1), read it and emit as event.
   */
  private pollForPrompts(): void {
    if (!this.gameSignal || !this.gameData) return;

    // Signal protocol:
    // 0 = IDLE, 1 = PROMPT_READY, 2 = RESPONSE_READY, 3 = PROMPT_ACKNOWLEDGED
    const poll = () => {
      if (!this.gameSignal || !this.gameData || !this.gameBuffer) return;

      const current = Atomics.load(this.gameSignal, 0);
      if (current === 1) { // SIGNAL_PROMPT_READY
        const len = Atomics.load(this.gameSignal, 1);
        const jsonBytes = this.gameData.slice(0, len);
        const jsonStr = new TextDecoder().decode(jsonBytes);

        // Acknowledge the prompt so we don't re-read it
        Atomics.store(this.gameSignal, 0, 3); // PROMPT_ACKNOWLEDGED
        Atomics.notify(this.gameSignal, 0);

        try {
          const prompt = JSON.parse(jsonStr);
          this.eventBus.emit("game:prompt", prompt);
        } catch (e) {
          console.error("[WorkerBridge] Failed to parse SAB prompt:", e);
        }
      }

      // Keep polling while game is active
      if (this.gameBuffer) {
        requestAnimationFrame(poll);
      }
    };

    requestAnimationFrame(poll);
  }

  /**
   * Poll the remote SAB for prompts and relay them via WebSocket.
   * When the remote player responds (via server:state_update), write to remote SAB.
   */
  private pollForRemotePrompts(): void {
    if (!this.remoteSignal || !this.remoteData) return;

    // Listen for relay responses from the remote player
    this.eventBus.on<{ from_player: string; state: Record<string, unknown> }>(
      "server:state_update",
      (payload) => {
        if (payload.state?.kind === "response" && this.remoteSignal && this.remoteData) {
          const action = payload.state.action;
          if (action) {
            const json = new TextEncoder().encode(JSON.stringify(action));
            Atomics.store(this.remoteSignal, 1, json.length);
            this.remoteData.set(json, 0);
            Atomics.store(this.remoteSignal, 0, 2); // RESPONSE_READY
            Atomics.notify(this.remoteSignal, 0);
          }
        }
      },
    );

    const poll = () => {
      if (!this.remoteSignal || !this.remoteData || !this.remoteBuffer) return;

      const current = Atomics.load(this.remoteSignal, 0);
      if (current === 1) { // PROMPT_READY
        const len = Atomics.load(this.remoteSignal, 1);
        const jsonBytes = this.remoteData.slice(0, len);
        const jsonStr = new TextDecoder().decode(jsonBytes);

        Atomics.store(this.remoteSignal, 0, 3); // ACKNOWLEDGED
        Atomics.notify(this.remoteSignal, 0);

        try {
          const prompt = JSON.parse(jsonStr);
          this.eventBus.emit("game:relay_prompt", prompt);
        } catch (e) {
          console.error("[WorkerBridge] Failed to parse remote SAB prompt:", e);
        }
      }

      if (this.remoteBuffer) {
        requestAnimationFrame(poll);
      }
    };

    requestAnimationFrame(poll);
  }

  /**
   * Write a response to the SharedArrayBuffer and wake the worker.
   */
  writeResponse(action: Record<string, unknown>): void {
    if (!this.gameSignal || !this.gameData) {
      console.error("[WorkerBridge] No SharedArrayBuffer available for response");
      return;
    }

    const json = new TextEncoder().encode(JSON.stringify(action));
    // Write length
    Atomics.store(this.gameSignal, 1, json.length);
    // Write data
    this.gameData.set(json, 0);
    // Signal response ready
    Atomics.store(this.gameSignal, 0, 2); // SIGNAL_RESPONSE_READY
    Atomics.notify(this.gameSignal, 0);
  }

  /**
   * Initialize the worker lazily.
   */
  async init(): Promise<void> {
    if (this.worker) return;

    if (this.initPromise) {
      return this.initPromise;
    }

    this.initPromise = this.doInit();
    return this.initPromise;
  }

  private async doInit(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        // Create worker using Vite's worker import pattern
        this.worker = new Worker(
          new URL("../workers/game-engine.worker.ts", import.meta.url),
          { type: "module" }
        );

        this.worker.onmessage = this.handleMessage.bind(this);
        this.worker.onerror = (e) => {
          console.error("[WorkerBridge] Worker error:", e);
          reject(new Error(`Worker error: ${e.message}`));
        };

        // Test the connection with a ping
        this.invoke("ping")
          .then(() => {
            console.log("[WorkerBridge] Worker initialized and responsive");
            resolve();
          })
          .catch(reject);
      } catch (error) {
        reject(error);
      }
    });
  }

  private handleMessage(e: MessageEvent<WorkerMessage>): void {
    const message = e.data;

    if (message.type === "response") {
      const pending = this.pendingRequests.get(message.requestId);
      if (pending) {
        this.pendingRequests.delete(message.requestId);
        if (message.error) {
          pending.reject(new Error(message.error));
        } else {
          pending.resolve(message.payload);
        }
      }
    } else if (message.type === "event") {
      // Forward events to the event bus
      this.eventBus.emit(message.event, message.payload);
    }
  }

  /**
   * Invoke a command on the worker.
   */
  async invoke<T>(
    command: string,
    args?: Record<string, unknown>
  ): Promise<T> {
    await this.init();

    if (!this.worker) {
      throw new Error("Worker not initialized");
    }

    const requestId = crypto.randomUUID();

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(requestId, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });

      const message: WorkerCommand = {
        type: "command",
        requestId,
        command,
        args,
      };

      this.worker!.postMessage(message);

      // Timeout after 30 seconds (except start_game which blocks for the whole game)
      const timeout = command === "start_game" ? 3600000 : 30000;
      setTimeout(() => {
        if (this.pendingRequests.has(requestId)) {
          this.pendingRequests.delete(requestId);
          reject(new Error(`Command timed out: ${command}`));
        }
      }, timeout);
    });
  }

  /**
   * Terminate the worker.
   */
  terminate(): void {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
    this.gameBuffer = null;
    this.gameSignal = null;
    this.gameData = null;
    this.remoteBuffer = null;
    this.remoteSignal = null;
    this.remoteData = null;
    this.pendingRequests.clear();
    this.initPromise = null;
  }
}

// ============================================================================
// Web Game API (WASM-based)
// ============================================================================

class WebGameApi implements IGameApi {
  private bridge: WorkerBridge;
  private serverApi: WebServerApi | null = null;
  private isMultiplayer = false;
  private isHost = false;
  private myPlayerSlot: string | null = null;

  constructor(bridge: WorkerBridge) {
    this.bridge = bridge;
  }

  /** Set server API reference for multiplayer relay */
  setServerApi(server: WebServerApi): void {
    this.serverApi = server;
  }

  async startGame(params: StartGameParams): Promise<string> {
    return this.bridge.invoke<string>("start_game", {
      deckList: params.deckList,
      startingLife: params.startingLife,
      commanderName: params.commanderName,
      opponentDeckList: params.opponentDeckList,
    });
  }

  async startMultiplayerGame(
    params: StartMultiplayerGameParams
  ): Promise<void> {
    this.isMultiplayer = true;
    this.isHost = params.localIsHost;
    this.myPlayerSlot = `player-${params.enginePlayerIndex}`;

    if (params.localIsHost) {
      // Host: run the engine in the worker with two SABs
      await this.bridge.invoke("start_multiplayer_game", {
        deckLists: params.deckLists,
        enginePlayerIndex: params.enginePlayerIndex,
        startingLife: params.startingLife,
      });
    }
    // Non-host: prompts arrive via game:remote_prompt WebSocket events.
    // Responses are sent via BroadcastState WebSocket relay.
  }

  async respond(params: RespondParams): Promise<void> {
    if (this.isMultiplayer && !this.isHost && this.serverApi) {
      // Non-host multiplayer: relay response via WebSocket to the host
      const fromPlayer = params.playerSlot ?? this.myPlayerSlot ?? "player-0";
      this.serverApi.broadcastState({
        kind: "response",
        fromPlayer,
        action: params.action,
      });
    } else if (this.bridge.gameBuffer) {
      // Host or single-player: write response to local SharedArrayBuffer
      this.bridge.writeResponse(params.action);
    } else {
      await this.bridge.invoke("respond", {
        action: params.action,
        playerSlot: params.playerSlot,
      });
    }
  }

  async endGame(): Promise<void> {
    this.isMultiplayer = false;
    this.isHost = false;
    this.myPlayerSlot = null;
    if (this.bridge.gameBuffer) {
      this.bridge.terminate();
      return;
    }
    await this.bridge.invoke("end_game");
  }

  async restoreSnapshot(params: RestoreSnapshotParams): Promise<void> {
    await this.bridge.invoke("restore_snapshot", {
      checkpointId: params.checkpointId,
    });
  }

  async getPresetDecks(): Promise<PresetDeckInfo[]> {
    return this.bridge.invoke<PresetDeckInfo[]>("get_preset_decks");
  }

  async validateDeckAvailability(deckList: Array<{ name: string }>): Promise<DeckAvailabilityResult> {
    return this.bridge.invoke<DeckAvailabilityResult>("validate_deck_availability", {
      deckList,
    });
  }

  async getPrompt(): Promise<unknown> {
    return this.bridge.invoke("get_prompt");
  }
}

// ============================================================================
// Web Storage API (localStorage-based, upgradeable to IndexedDB)
// ============================================================================

class WebStorageApi implements IStorageApi {
  private prefix = "openmagic:";

  async get<T>(key: string): Promise<T | null> {
    const item = localStorage.getItem(this.prefix + key);
    if (item === null) return null;
    try {
      return JSON.parse(item) as T;
    } catch {
      return null;
    }
  }

  async set<T>(key: string, value: T): Promise<void> {
    localStorage.setItem(this.prefix + key, JSON.stringify(value));
  }

  async remove(key: string): Promise<void> {
    localStorage.removeItem(this.prefix + key);
  }

  async keys(): Promise<string[]> {
    const allKeys = Object.keys(localStorage);
    return allKeys
      .filter((k) => k.startsWith(this.prefix))
      .map((k) => k.slice(this.prefix.length));
  }
}

// ============================================================================
// Web Event Bus (pure JS implementation)
// ============================================================================

class WebEventBus implements IEventBus {
  private listeners = new Map<string, Set<(payload: unknown) => void>>();

  on<T>(event: string, handler: (payload: T) => void): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }

    const handlers = this.listeners.get(event)!;
    const typedHandler = handler as (payload: unknown) => void;
    handlers.add(typedHandler);

    // Return unsubscribe function
    return () => {
      handlers.delete(typedHandler);
      if (handlers.size === 0) {
        this.listeners.delete(event);
      }
    };
  }

  emit<T>(event: string, payload: T): void {
    const handlers = this.listeners.get(event);
    if (handlers) {
      handlers.forEach((h) => h(payload));
    }
  }
}

// ============================================================================
// Web Server API (WebSocket-based multiplayer)
// ============================================================================

class WebServerApi implements IServerApi {
  private ws: WebSocket | null = null;
  private eventBus: WebEventBus;

  constructor(eventBus: WebEventBus) {
    this.eventBus = eventBus;

    // Relay prompts from the game engine to remote players via WebSocket
    eventBus.on<Record<string, unknown>>("game:relay_prompt", (prompt) => {
      this.broadcastState({
        kind: "prompt",
        forPlayer: "player-1", // TODO: support multiple remote players
        prompt,
      });
    });
  }

  async connect(params: ServerConnectParams): Promise<void> {
    this.disconnect();
    const scheme = params.port === 443 ? "wss" : "ws";
    const url = `${scheme}://${params.host}:${params.port}`;

    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(url);

      this.ws.onopen = () => {
        // Send authentication immediately
        this.send({
          type: "Authenticate",
          username: params.username,
          password: params.password,
        });
        resolve();
      };

      this.ws.onerror = () => {
        reject(new Error(`Failed to connect to ${url}`));
      };

      this.ws.onclose = () => {
        this.eventBus.emit("server:disconnected", {});
        this.ws = null;
      };

      this.ws.onmessage = (e: MessageEvent) => {
        if (typeof e.data !== "string") return;
        try {
          const msg = JSON.parse(e.data);
          this.handleServerMessage(msg);
        } catch {
          // Ignore malformed messages
        }
      };
    });
  }

  async disconnect(): Promise<void> {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  async listRooms(): Promise<void> {
    this.send({ type: "ListRooms" });
  }

  async listPlayers(): Promise<void> {
    this.send({ type: "ListPlayers" });
  }

  async createRoom(params: CreateRoomParams): Promise<void> {
    this.send({
      type: "CreateRoom",
      room_name: params.roomName,
      max_players: params.maxPlayers,
      format: params.format,
      hosted: params.hosted ?? false,
    });
  }

  async joinRoom(params: JoinRoomParams): Promise<void> {
    this.send({ type: "JoinRoom", room_id: params.roomId, observe: params.observe ?? false });
  }

  async leaveRoom(): Promise<void> {
    this.send({ type: "LeaveRoom" });
  }

  async setReady(params: SetReadyParams): Promise<void> {
    this.send({ type: "SetReady", ready: params.ready });
  }

  async setDeckSelection(params: SetDeckSelectionParams): Promise<void> {
    this.send({
      type: "SetDeckSelection",
      deck_name: params.deckName,
      deck_list: params.deckList,
      commander_name: params.commanderName,
    });
  }

  async startGame(): Promise<void> {
    this.send({ type: "StartGame" });
  }

  /** Broadcast game state to other players in the room */
  async broadcastState(state: Record<string, unknown>): Promise<void> {
    this.send({ type: "BroadcastState", state });
  }

  async sendRoomMessage(message: RoomRelayEnvelope): Promise<void> {
    this.send({ type: "BroadcastState", state: message });
  }

  async spawnAiBot(_params: SpawnAiBotParams): Promise<void> {
    throw new Error("Client-hosted AI bots are only available in the Tauri app.");
  }

  async removeAiBot(_username: string): Promise<void> {
    throw new Error("Client-hosted AI bots are only available in the Tauri app.");
  }

  private send(msg: Record<string, unknown>): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.error("[WebServerApi] Not connected");
      return;
    }
    this.ws.send(JSON.stringify(msg));
  }

  private handleServerMessage(msg: Record<string, unknown>): void {
    const type = msg.type as string;

    // Handle game relay envelopes in StateUpdate
    if (type === "StateUpdate" && msg.state) {
      const state = msg.state as Record<string, unknown>;
      const kind = state.kind as string | undefined;
      if (kind === "response") {
        // Route remote response to game store
        this.eventBus.emit("server:state_update", {
          from_player: msg.from_player,
          state,
        });
        return;
      } else if (isRoomRelayEnvelope(state)) {
        this.eventBus.emit("server:room_message", {
          from_player: msg.from_player,
          state,
        });
      } else if (kind === "prompt") {
        this.eventBus.emit("game:remote_prompt", state);
        return;
      } else if (kind === "log" && state.entry) {
        this.eventBus.emit("game:log", state.entry);
        return;
      } else if (kind === "snapshot" && state.entry) {
        this.eventBus.emit("game:snapshot", state.entry);
        return;
      }
    }

    // Handle error with not_in_room specially
    if (type === "Error" && msg.code === "not_in_room") {
      this.eventBus.emit("game:forced_end", {
        reason: "not_in_room",
        message: msg.message,
      });
    }

    // Map server message type to event name and payload
    const eventMap: Record<string, [string, unknown]> = {
      AuthResult: [
        "server:auth_result",
        {
          success: msg.success,
          player_id: msg.player_id,
          reconnected: msg.reconnected,
          error: msg.error,
        },
      ],
      RoomList: ["server:room_list", { rooms: msg.rooms }],
      PlayerList: ["server:player_list", { players: msg.players }],
      RoomCreated: [
        "server:room_created",
        { room_id: msg.room_id, room_name: msg.room_name },
      ],
      PlayerJoined: [
        "server:player_joined",
        { room_id: msg.room_id, username: msg.username },
      ],
      PlayerLeft: [
        "server:player_left",
        { room_id: msg.room_id, username: msg.username },
      ],
      PlayerConnected: [
        "server:player_connected",
        { username: msg.username },
      ],
      PlayerDisconnected: [
        "server:player_disconnected",
        { username: msg.username },
      ],
      ReadyStateChanged: [
        "server:ready_changed",
        { username: msg.username, ready: msg.ready },
      ],
      RoomUpdate: ["server:room_update", { room: msg.room }],
      GameStarted: [
        "server:game_started",
        {
          room_id: msg.room_id,
          player_order: msg.player_order,
          player_decks: msg.player_decks,
          starting_life: msg.starting_life,
        },
      ],
      StateUpdate: [
        "server:state_update",
        { from_player: msg.from_player, state: msg.state },
      ],
      TurnChanged: [
        "server:turn_changed",
        {
          from_player: msg.from_player,
          new_active_player: msg.new_active_player,
          turn_number: msg.turn_number,
        },
      ],
      Error: [
        "server:error",
        { code: msg.code, message: msg.message },
      ],
    };

    const mapping = eventMap[type];
    if (mapping) {
      this.eventBus.emit(mapping[0], mapping[1]);
    }
  }
}

// ============================================================================
// Web Platform
// ============================================================================

/**
 * Web platform implementation.
 * Used for browser-based deployments with WASM game engine.
 *
 * The game engine runs in a dedicated Web Worker to keep the UI responsive.
 * Communication happens via postMessage with a request/response protocol.
 */
export class WebPlatform implements IPlatformApi {
  readonly type = "web" as const;
  readonly game: IGameApi;
  readonly storage: IStorageApi;
  readonly events: WebEventBus;
  readonly server: IServerApi;

  private bridge: WorkerBridge;

  constructor() {
    this.events = new WebEventBus();
    this.storage = new WebStorageApi();
    this.bridge = new WorkerBridge(this.events);
    const serverApi = new WebServerApi(this.events);
    const gameApi = new WebGameApi(this.bridge);
    gameApi.setServerApi(serverApi);
    this.game = gameApi;
    this.server = serverApi;
  }

  /**
   * Initialize the platform (and worker) eagerly.
   * Call this at app startup for faster first game start.
   */
  async init(): Promise<void> {
    await this.bridge.init();
  }

  /**
   * Clean up resources.
   */
  dispose(): void {
    this.bridge.terminate();
  }

  isSupported(feature: PlatformFeature): boolean {
    switch (feature) {
      case "multiplayer":
        return true; // WebSocket-based multiplayer via forge-server
      case "native-dialogs":
        return false; // Browser has limited file system access
      case "system-tray":
        return false; // Not applicable to web
      case "auto-update":
        return false; // Web is always "updated" via cache
      case "offline-play":
        return false; // Browser path still depends on remote assets and no service worker exists
      default:
        return false;
    }
  }
}
