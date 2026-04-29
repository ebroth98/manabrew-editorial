import { useSetLookup } from "@/stores/useScryfallStore";
import { RARITY_LABEL, rarityToken } from "@/lib/limited.utils";
import { RaritySetSymbol } from "@/components/limited/RaritySetSymbol";
import type { DraftCard } from "@/types/limited";

interface RaritySetBadgeProps {
  card: DraftCard;
}

export function RaritySetBadge({ card }: RaritySetBadgeProps) {
  const setLookup = useSetLookup();
  if (!rarityToken(card.rarity)) return null;

  const set = card.setCode ? setLookup.get(card.setCode.toLowerCase()) : undefined;
  const label = RARITY_LABEL[card.rarity] ?? null;

  return (
    <span
      className="pointer-events-none absolute right-1 top-1 inline-flex h-5 w-5 items-center justify-center rounded-full bg-black/60 ring-1 ring-white/10"
      title={set ? `${label} · ${set.name}` : (label ?? undefined)}
    >
      <RaritySetSymbol rarity={card.rarity} setCode={card.setCode} className="h-3 w-3" />
    </span>
  );
}
