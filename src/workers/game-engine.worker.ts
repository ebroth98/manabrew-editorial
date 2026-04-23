/**
 * Web Worker for running the forge-wasm game engine.
 *
 * Uses SharedArrayBuffer + Atomics for blocking human player input.
 * The game loop runs synchronously in the worker, blocking on Atomics.wait()
 * when the human player needs to make a decision.
 */

import init, {
  wasm_init,
  echo,
  test_rng,
  test_foundation,
  load_card_bundle,
  parse_preset_decks,
  run_interactive_game,
  run_multiplayer_game,
} from "../wasm/forge_wasm";

// ============================================================================
// Types
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

interface PresetDeck {
  id: string;
  label: string;
  desc: string;
  color: string;
  coverCardName?: string;
  cards: Array<{ name: string; count: number; set?: string }>;
}

// ============================================================================
// State
// ============================================================================

/** 256KB SharedArrayBuffer for prompt/response communication */
const SAB_SIZE = 256 * 1024;

let wasmInitialized = false;
let cardsLoaded = false;
let presetDecks: PresetDeck[] = [];
let bundledCardNames = new Set<string>();
let gameSharedBuffer: SharedArrayBuffer | null = null;
let remoteSharedBuffer: SharedArrayBuffer | null = null;
let gameRunning = false;

// ============================================================================
// WASM and Data Initialization
// ============================================================================

async function initWasm(): Promise<void> {
  if (wasmInitialized) return;

  try {
    await init();
    wasm_init();
    wasmInitialized = true;
    console.log("[GameWorker] WASM initialized successfully");
    await loadCardData();
  } catch (error) {
    console.error("[GameWorker] Failed to initialize WASM:", error);
    throw error;
  }
}

async function loadCardData(): Promise<void> {
  if (cardsLoaded) return;

  try {
    console.log("[GameWorker] Loading card bundle...");

    const cardBundleResponse = await fetch("/wasm/cards-bundle.json");
    if (!cardBundleResponse.ok) {
      throw new Error(`Failed to fetch card bundle: ${cardBundleResponse.status}`);
    }
    const cardBundleText = await cardBundleResponse.text();
    const parsedBundle = JSON.parse(cardBundleText) as {
      cards?: Record<string, string>;
    };
    bundledCardNames = buildBundledCardNameIndex(parsedBundle.cards ?? {});

    const cardCount = load_card_bundle(cardBundleText);
    console.log(`[GameWorker] Loaded ${cardCount} cards into database`);

    const presetsResponse = await fetch("/wasm/preset-decks.json");
    if (!presetsResponse.ok) {
      throw new Error(`Failed to fetch preset decks: ${presetsResponse.status}`);
    }
    const presetsText = await presetsResponse.text();
    presetDecks = parse_preset_decks(presetsText) as PresetDeck[];
    console.log(`[GameWorker] Loaded ${presetDecks.length} preset decks`);

    cardsLoaded = true;
  } catch (error) {
    console.error("[GameWorker] Failed to load card data:", error);
    throw error;
  }
}

function normalizeCardName(name: string): string {
  return name
    .trim()
    .toLowerCase()
    .replace(/['"]/g, "")
    .replace(/[^a-z0-9]/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_|_$/g, "");
}

function buildBundledCardNameIndex(cards: Record<string, string>): Set<string> {
  const names = new Set<string>();

  for (const [filename, script] of Object.entries(cards)) {
    names.add(filename);

    const nameMatch = script.match(/^Name:(.+)$/m);
    if (nameMatch) {
      names.add(normalizeCardName(nameMatch[1]));
    }
  }

  return names;
}

function hasBundledCard(cardName: string): boolean {
  return bundledCardNames.has(normalizeCardName(cardName));
}

// ============================================================================
// Preset Deck Expansion
// ============================================================================

function expandDeckList(
  rawList: Array<{ name: string; count?: number }>,
): Array<{ name: string; count: number }> {
  if (rawList.length === 1 && rawList[0]) {
    const preset = presetDecks.find((p) => p.id === rawList[0]!.name);
    if (preset) {
      console.log(
        `[GameWorker] Expanding preset deck "${preset.id}" (${preset.cards.length} cards)`,
      );
      return preset.cards.map((c) => ({ name: c.name, count: c.count }));
    }
  }
  return rawList.map((c) => ({ name: c.name, count: c.count ?? 1 }));
}

function choosePresetCoverCardName(
  cards: Array<{ name: string; count: number; set?: string }>,
): string | undefined {
  return cards.find((card) => !/^([wburgc]|snow-)?basic land$/i.test(card.name))?.name
    ?? cards.find((card) => !/^(plains|island|swamp|mountain|forest|wastes)$/i.test(card.name))?.name
    ?? cards[0]?.name;
}

// ============================================================================
// Interactive Game Runner
// ============================================================================

/**
 * Start an interactive game. Sends the response to the main thread BEFORE
 * blocking on run_interactive_game(), so the UI can transition to the game view.
 */
function runInteractiveGame(
  requestId: string,
  args?: Record<string, unknown>,
): void {
  if (gameRunning) {
    postError(requestId, "Game already active. End current game first.");
    return;
  }

  const rawHumanDeck =
    (args?.deckList as Array<{ name: string; count?: number }>) || [];
  const rawAiDeck =
    (args?.opponentDeckList as Array<{ name: string; count?: number }>) ||
    rawHumanDeck;

  const humanDeck = { cards: expandDeckList(rawHumanDeck) };
  const aiDeck = { cards: expandDeckList(rawAiDeck) };
  const config = {
    starting_life: (args?.startingLife as number) || 20,
    commander_name: args?.commanderName as string | undefined,
  };

  console.log(
    "[GameWorker] Starting interactive game:",
    humanDeck.cards.length,
    "vs",
    aiDeck.cards.length,
  );

  // Allocate SharedArrayBuffer for prompt/response communication
  gameSharedBuffer = new SharedArrayBuffer(SAB_SIZE);
  gameRunning = true;

  // Send SAB to main thread so it can poll for prompts and write responses
  postEvent("game:sab", { buffer: gameSharedBuffer });

  // Send response BEFORE blocking — this lets the UI transition to game view
  postResponse(requestId, "game-started");

  // Run the game — this BLOCKS the worker thread!
  try {
    const result = run_interactive_game(
      humanDeck,
      aiDeck,
      config,
      gameSharedBuffer,
    );

    console.log("[GameWorker] Game completed:", result);
    gameRunning = false;
  } catch (e) {
    gameRunning = false;
    const msg = e instanceof Error ? e.message : String(e);
    console.error("[GameWorker] Game error:", msg);
    postEvent("game:forced_end", {
      reason: "worker_error",
      message: msg,
    });
  }
}

/**
 * Start a multiplayer game as host. Two SABs: one for local player, one for
 * remote player. Main thread routes remote SAB prompts via WebSocket.
 */
function runMultiplayerHostGame(
  requestId: string,
  args?: Record<string, unknown>,
): void {
  if (gameRunning) {
    postError(requestId, "Game already active.");
    return;
  }

  const deckLists = (args?.deckLists as Array<Array<{ name: string; count?: number }>>) || [];
  const localPlayerIndex = (args?.enginePlayerIndex as number) ?? 0;
  const startingLife = (args?.startingLife as number) || 20;

  const deck0 = { cards: expandDeckList(deckLists[0] || []) };
  const deck1 = { cards: expandDeckList(deckLists[1] || []) };
  const config = { starting_life: startingLife };

  console.log(
    "[GameWorker] Starting multiplayer game as host:",
    deck0.cards.length, "vs", deck1.cards.length,
    "local=player-" + localPlayerIndex,
  );

  gameSharedBuffer = new SharedArrayBuffer(SAB_SIZE);
  remoteSharedBuffer = new SharedArrayBuffer(SAB_SIZE);
  gameRunning = true;

  // Send both SABs to main thread — it routes local to UI, remote to WebSocket
  postEvent("game:sab", { buffer: gameSharedBuffer });
  postEvent("game:remote_sab", { buffer: remoteSharedBuffer });

  postResponse(requestId, "multiplayer-started");

  try {
    const result = run_multiplayer_game(
      deck0, deck1, config,
      gameSharedBuffer,
      remoteSharedBuffer,
      localPlayerIndex,
    );

    console.log("[GameWorker] Multiplayer game completed:", result);
    gameRunning = false;
  } catch (e) {
    gameRunning = false;
    const msg = e instanceof Error ? e.message : String(e);
    console.error("[GameWorker] Multiplayer game error:", msg);
    postEvent("game:forced_end", {
      reason: "worker_error",
      message: msg,
    });
  }
}

// ============================================================================
// Command Handlers
// ============================================================================

async function handleCommand(
  command: string,
  args?: Record<string, unknown>
): Promise<unknown> {
  await initWasm();

  switch (command) {
    case "ping":
      return "pong";

    case "echo":
      return echo(args?.message as string);

    case "test_rng":
      return test_rng();

    case "test_foundation":
      return test_foundation();

    case "start_game": {
      // Handled separately in onmessage — should not reach here
      throw new Error("start_game handled outside handleCommand");
    }

    case "respond": {
      // Responses are written directly to the SAB by the main thread,
      // not through worker commands. This is a no-op.
      return null;
    }

    case "end_game": {
      gameRunning = false;
      gameSharedBuffer = null;
      remoteSharedBuffer = null;
      console.log("[GameWorker] Game ending...");
      postEvent("game:ended", {});
      return null;
    }

    case "get_prompt": {
      // Prompts flow through the SAB, not through commands
      return null;
    }

    case "get_game_view": {
      return null;
    }

    case "restore_snapshot": {
      console.log("[GameWorker] Restore snapshot:", args?.checkpointId);
      return null;
    }

    case "get_preset_decks": {
      return presetDecks.map((deck) => ({
        id: deck.id,
        label: deck.label,
        desc: deck.desc,
        color: deck.color,
        coverCardName: deck.coverCardName ?? choosePresetCoverCardName(deck.cards),
      }));
    }

    case "validate_deck_availability": {
      const rawDeck = (args?.deckList as Array<{ name?: string; count?: number }> | undefined) ?? [];
      const normalizedDeck = rawDeck
        .filter((card): card is { name: string; count?: number } => typeof card.name === "string" && card.name.trim().length > 0)
        .map((card) => ({ name: card.name.trim(), count: card.count }));
      const expandedDeck = expandDeckList(normalizedDeck);
      const missingCards = Array.from(
        new Set(
          expandedDeck
            .map((card) => card.name?.trim())
            .filter((name): name is string => !!name)
            .filter((name) => !hasBundledCard(name)),
        ),
      ).sort((a, b) => a.localeCompare(b));

      return {
        supported: missingCards.length === 0,
        missingCards,
      };
    }

    default:
      throw new Error(`Unknown command: ${command}`);
  }
}

// ============================================================================
// Message Handling
// ============================================================================

function postResponse(requestId: string, payload?: unknown): void {
  const message: WorkerResponse = {
    type: "response",
    requestId,
    payload,
  };
  self.postMessage(message);
}

function postError(requestId: string, error: string): void {
  const message: WorkerResponse = {
    type: "response",
    requestId,
    error,
  };
  self.postMessage(message);
}

function postEvent(event: string, payload: unknown): void {
  const message: WorkerEvent = {
    type: "event",
    event,
    payload,
  };
  self.postMessage(message);
}

self.onmessage = async (e: MessageEvent<WorkerCommand>) => {
  const { type, requestId, command, args } = e.data;

  if (type !== "command") {
    console.warn("[GameWorker] Unknown message type:", type);
    return;
  }

  // These commands block the worker thread — send response before blocking
  if (command === "start_game") {
    try {
      await initWasm();
      runInteractiveGame(requestId, args);
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      console.error("[GameWorker] start_game error:", msg);
      postError(requestId, msg);
    }
    return;
  }

  if (command === "start_multiplayer_game") {
    try {
      await initWasm();
      runMultiplayerHostGame(requestId, args);
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error);
      console.error("[GameWorker] start_multiplayer_game error:", msg);
      postError(requestId, msg);
    }
    return;
  }

  try {
    const result = await handleCommand(command, args);
    postResponse(requestId, result);
  } catch (error) {
    const errorMessage =
      error instanceof Error ? error.message : String(error);
    console.error("[GameWorker] Command error:", command, errorMessage);
    postError(requestId, errorMessage);
  }
};

console.log("[GameWorker] Worker script loaded");
