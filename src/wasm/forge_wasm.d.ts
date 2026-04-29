/* tslint:disable */
/* eslint-disable */

/**
 * Verify WASM is working by echoing back a message.
 */
export function echo(msg: string): string;

/**
 * Get the number of cards in the database.
 */
export function get_card_count(): number;

/**
 * Get engine information.
 */
export function get_engine_info(): any;

export function get_token_count(): number;

/**
 * Look up a card by name to verify it exists.
 */
export function has_card(name: string): boolean;

/**
 * Check if the card database is loaded.
 */
export function is_card_db_loaded(): boolean;

export function is_token_db_loaded(): boolean;

export function limited_advance_gauntlet_round(gauntlet_id: string): any;

export function limited_cubecobra_url(cube_id_or_url: string): string;

export function limited_get_draft_state(session_id: string): any;

export function limited_get_gauntlet_state(gauntlet_id: string): any;

export function limited_get_sealed_pool(session_id: string): any;

export function limited_get_edition_info(set_code: string): any;

export function limited_get_winston_state(session_id: string): any;

export function limited_import_cube(request_json: any, body: string): any;

export function limited_list_chaos_themes(): any;

export function limited_list_conspiracy_hooks(): any;

export function limited_list_sealed_templates(): any;

export function limited_pick_card(session_id: string, card_name: string): any;

export function limited_undo_pick(session_id: string): any;

export function limited_record_gauntlet_outcome(gauntlet_id: string, won_game: boolean, match_over: boolean, match_won: boolean): any;

export function limited_start_booster_draft(setup_json: any): any;

export function limited_start_gauntlet_from_sealed(session_id: string, rounds: number): any;

export function limited_start_sealed(setup_json: any): any;

export function limited_start_winston(setup_json: any): any;

export function limited_winston_pass(session_id: string): any;

export function limited_winston_take(session_id: string): any;

/**
 * Load the card database from a JSON bundle string.
 *
 * This should be called once at startup with the contents of cards-bundle.json.
 * Returns the number of cards loaded.
 */
export function load_card_bundle(json_str: string): number;

export function load_token_bundle(json_str: string): number;

/**
 * Log a message to the browser console (for debugging).
 */
export function log(msg: string): void;

/**
 * Parse a game config from JSON.
 */
export function parse_config(config_json: any): any;

/**
 * Parse a deck list from JSON.
 *
 * Returns a summary of the parsed deck for verification.
 */
export function parse_deck(deck_json: any): any;

/**
 * Parse preset decks JSON.
 */
export function parse_preset_decks(json_str: string): any;

/**
 * Run an interactive game with a human player (blocking on Atomics.wait).
 *
 * This function blocks the worker thread until the game is complete.
 * The human player's prompts are written to the SharedArrayBuffer,
 * and the worker blocks until the main thread provides a response.
 *
 * Call this from a Web Worker — it will block the thread.
 */
export function run_interactive_game(human_deck_json: any, ai_deck_json: any, config_json: any, shared_buffer: any): any;

/**
 * Run a multiplayer game with two players using separate SharedArrayBuffers.
 *
 * Player 0 (local) uses `local_buffer` — prompts shown in UI.
 * Player 1 (remote) uses `remote_buffer` — prompts relayed via WebSocket.
 * Both block on Atomics.wait() sequentially (never concurrently).
 */
export function run_multiplayer_game(player0_deck_json: any, player1_deck_json: any, config_json: any, local_buffer: any, remote_buffer: any, local_player_index: number): any;

/**
 * Test that forge-foundation types work.
 */
export function test_foundation(): any;

/**
 * Test that the RNG works in WASM.
 */
export function test_rng(): any;

/**
 * Initialize the WASM module. Call this once at startup.
 */
export function wasm_init(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly wasm_init: () => void;
    readonly log: (a: number, b: number) => void;
    readonly load_card_bundle: (a: number, b: number) => [number, number, number];
    readonly load_token_bundle: (a: number, b: number) => [number, number, number];
    readonly is_card_db_loaded: () => number;
    readonly get_card_count: () => number;
    readonly is_token_db_loaded: () => number;
    readonly get_token_count: () => number;
    readonly has_card: (a: number, b: number) => number;
    readonly parse_preset_decks: (a: number, b: number) => [number, number, number];
    readonly limited_list_sealed_templates: () => [number, number, number];
    readonly limited_list_chaos_themes: () => [number, number, number];
    readonly limited_list_conspiracy_hooks: () => [number, number, number];
    readonly limited_start_sealed: (a: any) => [number, number, number];
    readonly limited_get_sealed_pool: (a: number, b: number) => [number, number, number];
    readonly limited_get_edition_info: (a: number, b: number) => [number, number, number];
    readonly limited_start_booster_draft: (a: any) => [number, number, number];
    readonly limited_pick_card: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly limited_undo_pick: (a: number, b: number) => [number, number, number];
    readonly limited_get_draft_state: (a: number, b: number) => [number, number, number];
    readonly limited_start_winston: (a: any) => [number, number, number];
    readonly limited_winston_take: (a: number, b: number) => [number, number, number];
    readonly limited_winston_pass: (a: number, b: number) => [number, number, number];
    readonly limited_get_winston_state: (a: number, b: number) => [number, number, number];
    readonly limited_start_gauntlet_from_sealed: (a: number, b: number, c: number) => [number, number, number];
    readonly limited_record_gauntlet_outcome: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly limited_advance_gauntlet_round: (a: number, b: number) => [number, number, number];
    readonly limited_get_gauntlet_state: (a: number, b: number) => [number, number, number];
    readonly limited_cubecobra_url: (a: number, b: number) => [number, number, number, number];
    readonly limited_import_cube: (a: any, b: number, c: number) => [number, number, number];
    readonly get_engine_info: () => any;
    readonly echo: (a: number, b: number) => [number, number];
    readonly parse_deck: (a: any) => [number, number, number];
    readonly parse_config: (a: any) => [number, number, number];
    readonly test_rng: () => any;
    readonly test_foundation: () => any;
    readonly run_interactive_game: (a: any, b: any, c: any, d: any) => [number, number, number];
    readonly run_multiplayer_game: (a: any, b: any, c: any, d: any, e: any, f: number) => [number, number, number];
    readonly __wbindgen_malloc_command_export: (a: number, b: number) => number;
    readonly __wbindgen_realloc_command_export: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store_command_export: (a: number) => void;
    readonly __externref_table_alloc_command_export: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free_command_export: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc_command_export: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
