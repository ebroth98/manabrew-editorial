import { cn } from "@/lib/utils";
import type { GameLogEntryType, GameLogEntry } from "@/types/gameLog";

interface ActionLogProps {
  gameLog: GameLogEntry[];
  resolveCardName: (cardId: string) => string;
  resolvePlayerName: (playerId: string) => string;
  onHoverLogCard: (cardId: string | null, event?: React.MouseEvent) => void;
}

export function ActionLog({
  gameLog,
  resolveCardName,
  resolvePlayerName,
  onHoverLogCard,
}: ActionLogProps) {
  const visibleLog = gameLog.filter((entry) => entry.entryType !== "rule");

  const typeLabel: Record<GameLogEntryType, string> = {
    info: "INFO",
    action: "ACTION",
    stack: "STACK",
    priority: "PRIO",
    rule: "RULE",
    warning: "WARN",
  };

  const typeClass: Record<GameLogEntryType, string> = {
    info: "bg-muted text-muted-foreground",
    action: "bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300",
    stack: "bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-300",
    priority: "bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300",
    rule: "bg-purple-100 text-purple-800 dark:bg-purple-950 dark:text-purple-300",
    warning: "bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-300",
  };

  const formatTs = (timestampMs: number) =>
    new Date(timestampMs).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

  const isResolveEntry = (entryType: GameLogEntryType, message: string) =>
    entryType === "stack" && /\bresolved?\b/i.test(message);

  const isTurnEntry = (message: string) => /^TURN\b/i.test(message);

  if (visibleLog.length === 0) {
    return (
      <div className="rounded-lg p-2.5 min-h-0 flex-1 flex flex-col bg-muted/20">
        <p className="text-xs font-semibold text-muted-foreground mb-2">Game Log</p>
        <p className="text-xs text-muted-foreground italic">No log entries yet.</p>
      </div>
    );
  }

  return (
    <div className="rounded-lg p-2.5 min-h-0 flex-1 flex flex-col bg-muted/20">
      <p className="text-xs font-semibold text-muted-foreground mb-2">Game Log</p>
      <div className="min-h-0 flex-1 overflow-y-auto text-xs text-muted-foreground flex flex-col-reverse pr-1">
        {visibleLog.slice(-200).reverse().map((entry, i) => (
          <div
            key={i}
            className={cn(
              "py-1 border-b border-border/40 last:border-b-0",
              entry.entryType === "warning" && "text-red-400 font-semibold",
            )}
            onMouseEnter={(e) => onHoverLogCard(entry.cardId ?? null, e)}
            onMouseLeave={() => onHoverLogCard(null)}
          >
            <div className="flex items-center gap-1.5 mb-0.5">
              <span
                className={cn(
                  "px-1 py-0.5 rounded text-[10px] font-semibold",
                  isResolveEntry(entry.entryType, entry.message)
                    ? "bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300"
                    : isTurnEntry(entry.message)
                      ? "bg-cyan-100 text-cyan-900 dark:bg-cyan-950 dark:text-cyan-300"
                    : typeClass[entry.entryType],
                )}
              >
                {isResolveEntry(entry.entryType, entry.message)
                  ? "RESOLVE"
                  : isTurnEntry(entry.message)
                    ? "TURN"
                  : typeLabel[entry.entryType]}
              </span>
              <span className="text-[10px] text-muted-foreground/80">
                {formatTs(entry.timestampMs)}
              </span>
              {entry.playerId && (
                <span className="text-[10px] text-muted-foreground/80">
                  {resolvePlayerName(entry.playerId)}
                </span>
              )}
            </div>
            {(entry.sourceCardId || entry.targetCardId) && (
              <div className="flex items-center gap-1 mb-0.5">
                {entry.sourceCardId && (
                  <span
                    className="px-1 py-0.5 rounded text-[10px] bg-sky-100 text-sky-800 dark:bg-sky-950 dark:text-sky-300 cursor-help"
                    onMouseEnter={(e) => onHoverLogCard(entry.sourceCardId!, e)}
                    onMouseLeave={() => onHoverLogCard(null)}
                  >
                    SRC: {resolveCardName(entry.sourceCardId)}
                  </span>
                )}
                {entry.targetCardId && (
                  <span
                    className="px-1 py-0.5 rounded text-[10px] bg-rose-100 text-rose-800 dark:bg-rose-950 dark:text-rose-300 cursor-help"
                    onMouseEnter={(e) => onHoverLogCard(entry.targetCardId!, e)}
                    onMouseLeave={() => onHoverLogCard(null)}
                  >
                    TGT: {resolveCardName(entry.targetCardId)}
                  </span>
                )}
              </div>
            )}
            <div className={entry.entryType === "warning" ? "whitespace-pre-wrap break-all" : undefined}>{entry.message}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
