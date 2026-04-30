// SPDX-License-Identifier: GPL-3.0-or-later

import type { GameView } from "@/types/openmagic";

export interface GameSnapshotEntry {
  checkpointId: number;
  label: string;
  timestampMs: number;
  gameView: GameView;
}

export function normalizeSnapshotPayload(payload: unknown): GameSnapshotEntry {
  const p = (payload ?? {}) as Partial<GameSnapshotEntry> & Record<string, unknown>;
  const checkpointId =
    typeof p.checkpointId === "number"
      ? p.checkpointId
      : typeof p.checkpoint_id === "number"
        ? (p.checkpoint_id as number)
        : 0;
  const label = typeof p.label === "string" ? p.label : `Checkpoint ${checkpointId}`;
  const timestampMs =
    typeof p.timestampMs === "number"
      ? p.timestampMs
      : typeof p.timestamp_ms === "number"
        ? (p.timestamp_ms as number)
        : Date.now();
  const gameView = (p.gameView ?? p.game_view ?? null) as GameView | null;
  return {
    checkpointId,
    label,
    timestampMs,
    gameView: gameView as GameView,
  };
}
