import { ManaSymbols } from "@/components/game/ManaSymbols";
import { cn } from "@/lib/utils";

const CHIP_BASE =
  "text-[8px] font-bold uppercase bg-black/60 text-white px-1 py-px rounded leading-none";

/** Render a keyword chip — if it contains a colon, the part after is a mana cost. */
export function KeywordChip({ kw }: { kw: string }) {
  const colonIdx = kw.indexOf(":");
  if (colonIdx === -1) {
    return <span className={CHIP_BASE}>{kw}</span>;
  }
  const label = kw.slice(0, colonIdx);
  const cost = kw.slice(colonIdx + 1);
  return (
    <span className={cn("inline-flex items-center gap-0.5", CHIP_BASE)}>
      {label}
      <ManaSymbols cost={cost} size="sm" />
    </span>
  );
}

export function KeywordChips({ keywords }: { keywords: string[] }) {
  if (!keywords || keywords.length === 0) return null;
  const visible = keywords.slice(0, 4);
  const hidden = keywords.length - visible.length;
  // Anchor the chip strip just below the MTG title line (same 13% band
  // the status badges use) so the card name + mana cost stay legible.
  return (
    <div className="absolute top-[10%] left-1 right-1 flex flex-wrap gap-0.5 z-10">
      {visible.map((kw) => (
        <KeywordChip key={kw} kw={kw} />
      ))}
      {hidden > 0 && <span className={CHIP_BASE}>+{hidden}</span>}
    </div>
  );
}
