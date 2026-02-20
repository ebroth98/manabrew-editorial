import { useQuery } from "@tanstack/react-query";
import { getCardByName } from "@/api/scryfall";

/**
 * Returns a Scryfall image URL for the given card name.
 * If the card already has an imageUrl stored, skips the fetch.
 */
export function useCardImage(name: string, existingUrl?: string) {
  return useQuery({
    queryKey: ["card-image", name],
    queryFn: async () => {
      const card = await getCardByName(name);
      return card.image_uris?.normal ?? card.image_uris?.large ?? null;
    },
    enabled: !!name && !existingUrl,
    staleTime: Infinity, // card images never change
    gcTime: 1000 * 60 * 60, // keep in cache 1 hour
    retry: false,
  });
}
