import { useQuery } from "@tanstack/react-query";
import { getCardByName, getTokenBySetAndNumber } from "@/api/scryfall";

/**
 * Returns a Scryfall image URL for the given card name.
 * If the card already has an imageUrl stored, skips the fetch.
 * For tokens, uses the set code + collector number for a direct Scryfall lookup.
 * Pass setCode to get a specific printing for non-token cards.
 */
export function useCardImage(
  name: string,
  existingUrl?: string,
  isToken?: boolean,
  color?: string,
  setCode?: string,
  cardNumber?: string,
  size: "small" | "normal" | "large" | "png" = "normal"
) {
  return useQuery({
    queryKey: ["card-image", name, isToken ? "token" : "card", color ?? "", setCode ?? "", cardNumber ?? "", size],
    queryFn: async () => {
      let card;
      if (isToken && setCode && cardNumber) {
        card = await getTokenBySetAndNumber(setCode, cardNumber);
      } else {
        card = await getCardByName(name, setCode);
      }
      // Double-faced cards return card_faces instead of top-level image_uris.
      // Find the face matching the current name (works for both front and back face).
      if (card.card_faces) {
        const face = card.card_faces.find(
          (f) => f.name.toLowerCase() === name.toLowerCase(),
        );
        if (face?.image_uris) {
          return face.image_uris[size] ?? face.image_uris.normal ?? face.image_uris.large ?? null;
        }
      }
      return card.image_uris?.[size] ?? card.image_uris?.normal ?? card.image_uris?.large ?? null;
    },
    enabled: !!name && !existingUrl,
    staleTime: Infinity, // card images never change
    gcTime: 1000 * 60 * 60, // keep in cache 1 hour
    retry: false,
  });
}
