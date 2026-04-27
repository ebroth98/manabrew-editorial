import { cn } from "@/lib/utils";
import { useCard } from "@/stores/useScryfallStore";
import type { Card } from "@/types/openmagic";

interface DeckCoverImageProps {
  cover: Card | null | undefined;
  alt?: string;
  className?: string;
  fallbackClassName?: string;
}

export function DeckCoverImage({ cover, alt, className }: DeckCoverImageProps) {
  const card = useCard(cover);
  if (!card) return null;
  return (
    <img
      src={card.uris.art_crop}
      alt={alt ?? cover?.name ?? "Deck cover"}
      loading="lazy"
      className={cn(
        "absolute inset-0 h-full w-full object-cover",
        "transition-transform duration-300 ease-out group-hover:scale-110",
        className,
      )}
    />
  );
}
