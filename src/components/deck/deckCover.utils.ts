import { useQuery } from "@tanstack/react-query";
import { getCardByName } from "@/api/scryfall";
import type { Card, Deck } from "@/types/openmagic";
import type { ScryfallCard } from "@/types/scryfall";

export interface DeckCoverSource {
  cardName: string;
  isDoubleFaced?: boolean;
  coverCardFace?: 0 | 1;
}

export function resolveCoverCard(deck: Deck): Card | undefined {
  const allCards = [...deck.cards, ...(deck.commanders ?? [])];
  if (deck.coverCardName) {
    const found = allCards.find((card) => card.name === deck.coverCardName);
    if (found) return found;
  }
  return deck.commanders?.[0] ?? deck.cards[0];
}

function resolveArtCropUrl(card: ScryfallCard, cover?: DeckCoverSource): string | undefined {
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
