import { cn } from "@/lib/utils";

interface LibraryZoneTileProps {
  count: number;
  onClick?: () => void;
  label?: string;
}

export function LibraryZoneTile({
  count,
  onClick,
  label = "Lib",
}: LibraryZoneTileProps) {
  return (
    <div className="flex flex-col items-center gap-0.5">
      <div className="relative h-16 w-10">
        <span className="absolute left-1 top-1 h-14 w-8 rounded-md border border-amber-500/35 bg-blue-950/40" />
        <span className="absolute left-0.5 top-0.5 h-14 w-8 rounded-md border border-amber-500/55 bg-blue-950/55" />
        <button
          className={cn(
            "absolute left-0 top-0 h-14 w-8 rounded-md text-card-foreground transition-colors",
            "bg-blue-950/70 border-2 border-amber-500/80 shadow-[inset_0_0_0_1px_rgba(59,130,246,0.35)]",
            onClick ? "hover:bg-blue-900/70" : "opacity-95",
          )}
          onClick={onClick}
          disabled={!onClick}
          title="Library"
        >
          <span className="absolute inset-[4px] rounded-[4px] border border-blue-300/30 bg-blue-900/35" />
          <span className="relative z-10 text-base font-bold leading-none text-amber-100">
            {count}
          </span>
        </button>
      </div>
      <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
        {label}
      </span>
    </div>
  );
}
