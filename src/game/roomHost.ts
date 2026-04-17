import { getPlatform } from "@/platform";
import type { GameView } from "@/types/openmagic";
import type { ManualTabletopAction, SeatController } from "./runtime.types";

const ROOM_HOST_ENVELOPE_KIND = "roomHost";

export type RoomHostMode = "authoritative-host" | "relay-client";

export type RoomHostPayload =
  | {
      type: "manualState";
      gameView: GameView;
    }
  | {
      type: "manualAction";
      action: ManualTabletopAction;
    };

export interface RoomHostEnvelope {
  kind: typeof ROOM_HOST_ENVELOPE_KIND;
  fromPlayer: string;
  mode: RoomHostMode;
  payload: RoomHostPayload;
}

export interface BroadcastRoomHostConfig {
  localPlayerSlot: string;
  mode: RoomHostMode;
  seats: SeatController[];
}

export class BroadcastRoomHost {
  readonly localPlayerSlot: string;
  readonly mode: RoomHostMode;
  readonly seats: SeatController[];

  constructor(config: BroadcastRoomHostConfig) {
    this.localPlayerSlot = config.localPlayerSlot;
    this.mode = config.mode;
    this.seats = config.seats;
  }

  async broadcastManualState(gameView: GameView): Promise<void> {
    await this.broadcast({
      type: "manualState",
      gameView,
    });
  }

  async broadcastManualAction(action: ManualTabletopAction): Promise<void> {
    await this.broadcast({
      type: "manualAction",
      action,
    });
  }

  subscribe(
    handler: (envelope: RoomHostEnvelope) => void,
  ): () => void {
    return getPlatform().events.on<{ from_player?: string; state?: unknown }>(
      "server:state_update",
      (payload) => {
        const envelope = payload.state;
        if (!isRoomHostEnvelope(envelope)) return;
        if (envelope.fromPlayer === this.localPlayerSlot) return;
        handler(envelope);
      },
    );
  }

  private async broadcast(payload: RoomHostPayload): Promise<void> {
    const server = getPlatform().server;
    if (!server) {
      throw new Error("Room hosting requires a server connection.");
    }
    await server.broadcastState({
      kind: ROOM_HOST_ENVELOPE_KIND,
      fromPlayer: this.localPlayerSlot,
      mode: this.mode,
      payload,
    });
  }
}

export function isRoomHostEnvelope(
  value: unknown,
): value is RoomHostEnvelope {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<RoomHostEnvelope>;
  return (
    candidate.kind === ROOM_HOST_ENVELOPE_KIND &&
    typeof candidate.fromPlayer === "string" &&
    (candidate.mode === "authoritative-host" ||
      candidate.mode === "relay-client") &&
    !!candidate.payload &&
    typeof candidate.payload === "object"
  );
}
