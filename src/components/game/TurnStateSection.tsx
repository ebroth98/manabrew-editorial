import { cn } from "@/lib/utils";

interface TurnStateSectionProps {
  turn: number;
  activePlayerName: string;
  isMyTurn: boolean;
  isMyPriority: boolean;
}

export function TurnStateSection({ turn, activePlayerName, isMyTurn, isMyPriority }: TurnStateSectionProps) {
  return (
    <div className="rounded-lg px-2.5 py-2 bg-muted/25">
      <div className="flex items-center gap-1.5">
        <p className="text-sm font-semibold">Turn {turn} -</p>
        <p className={cn("text-sm font-medium", isMyTurn ? "text-green-700 dark:text-green-300" : "text-amber-700 dark:text-amber-300")}>
          {isMyTurn ? "Your turn" : `${activePlayerName}'s turn`}
        </p>
        {isMyPriority && (
          <span className="ml-1 text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0 bg-purple-100 text-purple-700 dark:bg-purple-950/40 dark:text-purple-300 animate-pulse">
            PRIORITY
          </span>
        )}
      </div>
    </div>
  );
}
