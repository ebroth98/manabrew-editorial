/**
 * Resolve a player's seat color from the current theme.
 *
 * The human player sits in the `self` slot; opponents fan out into the
 * `opponent1` / `opponent2` / `opponent3` palette in turn order. Used
 * by every dice modal so a roll's accent color always matches the
 * roller's seat — no special-casing per modal.
 */

const OPPONENT_SEATS = ["opponent1", "opponent2", "opponent3"] as const;
type OpponentSeat = (typeof OPPONENT_SEATS)[number];

export interface PlayerColorPalette {
  self: string;
  opponent1: string;
  opponent2: string;
  opponent3: string;
}

export interface PlayerSeatInfo {
  id: string;
  isHuman: boolean;
}

export function buildPlayerColorMap(
  players: PlayerSeatInfo[],
  palette: PlayerColorPalette,
): Map<string, string> {
  const map = new Map<string, string>();
  const me = players.find((p) => p.isHuman);
  if (me) map.set(me.id, palette.self);
  const opponents = players.filter((p) => !p.isHuman);
  opponents.forEach((opp, i) => {
    const seat: OpponentSeat = OPPONENT_SEATS[i] ?? "opponent1";
    map.set(opp.id, palette[seat]);
  });
  return map;
}

export function resolvePlayerColor(
  playerId: string | undefined,
  players: PlayerSeatInfo[],
  palette: PlayerColorPalette,
): string | undefined {
  if (!playerId) return undefined;
  return buildPlayerColorMap(players, palette).get(playerId);
}
