import { useMemo, useState } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Modal } from "@/components/game/modals/Modal";
import { useTheme } from "@/hooks/useTheme";
import { DieFaceStatic } from "./DieFaceStatic";
import { buildPlayerColorMap, type PlayerSeatInfo } from "./playerColor";

/** Must match the `--animate-dice-roll` duration in `src/index.css`. */
const ANIMATION_DURATION_MS = 2000;

export interface FirstPlayerRollEntry {
  playerId: string;
  playerName: string;
  value: number;
}

interface FirstPlayerRollFeedbackProps {
  sides: number;
  rolls: FirstPlayerRollEntry[];
  winnerPlayerId: string;
  /** Players from the current game view; used to assign self/opponent colors. */
  players: PlayerSeatInfo[];
  onAcknowledge: () => void;
}

interface DieAnimationParams {
  spinDeg: number;
  delayMs: number;
}

/**
 * Display-only modal shown at the start of every game. Each player
 * rolled a d20 simultaneously; the highest value goes first. Animates
 * every die in parallel, colored by player, then waits for the player
 * to click Continue.
 */
export function FirstPlayerRollFeedback({
  sides,
  rolls,
  winnerPlayerId,
  players,
  onAcknowledge,
}: FirstPlayerRollFeedbackProps) {
  const themeColors = useTheme().gameTheme;
  const colorByPlayerId = useMemo(
    () => buildPlayerColorMap(players, themeColors.playerColors),
    [players, themeColors.playerColors],
  );

  // Stable randomized motion per die (lazy init so re-renders don't re-roll).
  const [params] = useState<DieAnimationParams[]>(() =>
    Array.from({ length: rolls.length }, generateParams),
  );

  const winner = rolls.find((r) => r.playerId === winnerPlayerId);

  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <div role="dialog" aria-modal="true" aria-labelledby="first-player-roll-title">
        <Modal.Header>
          <div>
            <h2 id="first-player-roll-title" className="font-semibold text-base">
              Roll for first player
            </h2>
            <p className="text-xs text-muted-foreground">Highest d{sides} goes first</p>
          </div>
        </Modal.Header>

        <div className="px-4 py-6 flex items-end justify-center gap-6 flex-wrap">
          {rolls.map((roll, index) => {
            const accent = colorByPlayerId.get(roll.playerId);
            return (
              <div key={roll.playerId} className="flex flex-col items-center gap-2">
                <span
                  className="text-xs font-medium uppercase tracking-wide"
                  style={accent ? { color: accent } : undefined}
                >
                  {roll.playerName}
                </span>
                <div
                  className="animate-dice-roll"
                  style={
                    {
                      animationDelay: `${params[index]?.delayMs ?? 0}ms`,
                      ["--dice-spin" as string]: `${params[index]?.spinDeg ?? 540}deg`,
                    } as React.CSSProperties
                  }
                >
                  <DieFaceStatic
                    sides={sides}
                    value={roll.value}
                    size="lg"
                    accentColor={accent}
                    ariaLabel={`${roll.playerName} rolled ${roll.value}`}
                  />
                </div>
              </div>
            );
          })}
        </div>

        {winner && (
          <div
            className={cn(
              "px-4 pb-4 text-sm font-semibold text-center",
              "animate-in fade-in duration-300",
            )}
            style={{ animationDelay: `${ANIMATION_DURATION_MS}ms`, animationFillMode: "both" }}
          >
            <span className="text-muted-foreground">First player: </span>
            <span style={{ color: colorByPlayerId.get(winner.playerId) }}>{winner.playerName}</span>
          </div>
        )}

        <Modal.Footer>
          <Button size="sm" onClick={onAcknowledge}>
            Continue
          </Button>
        </Modal.Footer>
      </div>
    </Modal>
  );
}

function generateParams(): DieAnimationParams {
  const turns = 2 + Math.random();
  const sign = Math.random() < 0.5 ? -1 : 1;
  return {
    spinDeg: Math.round(turns * 360 * sign),
    delayMs: Math.floor(Math.random() * 80),
  };
}
