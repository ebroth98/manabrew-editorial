/** Type stubs for the wasm-pack generated forge_wasm module. */
declare const init: () => Promise<void>;
export default init;
export function wasm_init(): void;
export function echo(input: string): string;
export function test_rng(): void;
export function test_foundation(): void;
export function load_card_bundle(data: string): number;
export function parse_preset_decks(data: string): unknown;
export function run_interactive_game(
  deck1: unknown,
  deck2: unknown,
  config: unknown,
  shared_buffer: SharedArrayBuffer,
): unknown;
export function run_multiplayer_game(
  deck0: unknown,
  deck1: unknown,
  config: unknown,
  local_sab: SharedArrayBuffer,
  remote_sab: SharedArrayBuffer,
  local_player_index: number,
): unknown;
