import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getCardByName } from "@/api/scryfall";
import { cn } from "@/lib/utils";
import type { Card, Deck } from "@/types/openmagic";
import type { ScryfallCard } from "@/types/scryfall";

export interface DeckCoverSource {
  cardName: string;
  isDoubleFaced?: boolean;
  coverCardFace?: 0 | 1;
}

interface DeckCoverImageProps {
  cover?: DeckCoverSource;
  alt?: string;
  className?: string;
  fallbackClassName?: string;
}

export function resolveCoverCard(deck: Deck): Card | undefined {
  const allCards = [...deck.cards, ...(deck.commanders ?? [])];
  if (deck.coverCardName) {
    const found = allCards.find((card) => card.name === deck.coverCardName);
    if (found) return found;
  }
  return deck.commanders?.[0] ?? deck.cards[0];
}

function resolveArtCropUrl(
  card: ScryfallCard,
  cover?: DeckCoverSource,
): string | undefined {
  const wantsBackFace = cover?.isDoubleFaced && cover.coverCardFace === 1;
  if (wantsBackFace) {
    return card.card_faces?.[1]?.image_uris?.art_crop;
  }
  return card.image_uris?.art_crop ?? card.card_faces?.[0]?.image_uris?.art_crop;
}

export function resolveDeckCoverSource(deck: Deck): DeckCoverSource | undefined {
  const coverCard = resolveCoverCard(deck);
  if (!coverCard) return undefined;

  return {
    cardName: coverCard.name,
    isDoubleFaced: coverCard.isDoubleFaced,
    coverCardFace: deck.coverCardFace,
  };
}

export function resolvePresetDeckCoverSource(coverCardName?: string): DeckCoverSource | undefined {
  if (!coverCardName) return undefined;
  return { cardName: coverCardName };
}

export function useDeckCoverUrl(cover?: DeckCoverSource): string | undefined {
  const { data: artUrl } = useQuery({
    queryKey: ["deck-cover-art", cover?.cardName, cover?.coverCardFace],
    queryFn: async () => {
      const scryfall = await getCardByName(cover!.cardName);
      return resolveArtCropUrl(scryfall, cover) ?? null;
    },
    enabled: !!cover?.cardName,
    staleTime: Infinity,
    gcTime: 1000 * 60 * 60,
    retry: false,
  });

  return artUrl ?? undefined;
}

export function DeckCoverImage({
  cover,
  alt,
  className,
  fallbackClassName,
}: DeckCoverImageProps) {
  const [artError, setArtError] = useState(false);
  const artUrl = useDeckCoverUrl(cover);

  useEffect(() => {
    setArtError(false);
  }, [artUrl]);

  if (artUrl && !artError) {
    return (
      <img
        src={artUrl}
        alt={alt ?? cover?.cardName ?? "Deck cover"}
        loading="lazy"
        className={cn(
          "absolute inset-0 h-full w-full object-cover",
          "transition-transform duration-300 ease-out group-hover:scale-110",
          className,
        )}
        onError={() => setArtError(true)}
      />
    );
  }

  return (
    <div
      className={cn(
        "absolute inset-0 bg-gradient-to-br from-muted-foreground/5 to-muted-foreground/20",
        fallbackClassName,
      )}
    />
  );
}
