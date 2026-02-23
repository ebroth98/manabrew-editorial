import { useQuery } from "@tanstack/react-query";
import { getCardByName, getTokenByName } from "@/api/scryfall";

/**
 * Returns a Scryfall image URL for the given card name.
 * If the card already has an imageUrl stored, skips the fetch.
 * Pass isToken=true to search the token database instead of named cards.
 * Pass color (engine format: "W", "WU", "C") to disambiguate same-named tokens.
 * Pass setCode to get a specific printing.
 */
export function useCardImage(name: string, existingUrl?: string, isToken?: boolean, color?: string, setCode?: string) {
  return useQuery({
    queryKey: ["card-image", name, isToken ? "token" : "card", color ?? "", setCode ?? ""],
    queryFn: async () => {
      const card = isToken ? await getTokenByName(name, color) : await getCardByName(name, setCode);
      // Double-faced cards return card_faces instead of top-level image_uris.
      // Find the face matching the current name (works for both front and back face).
      if (card.card_faces) {
        const face = card.card_faces.find(
          (f) => f.name.toLowerCase() === name.toLowerCase(),
        );
        if (face?.image_uris) {
          return face.image_uris.normal ?? face.image_uris.large ?? null;
        }
      }
      return card.image_uris?.normal ?? card.image_uris?.large ?? null;
    },
    enabled: !!name && !existingUrl,
    staleTime: Infinity, // card images never change
    gcTime: 1000 * 60 * 60, // keep in cache 1 hour
    retry: false,
  });
}
