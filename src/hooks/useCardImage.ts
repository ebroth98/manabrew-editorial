import { useQuery } from "@tanstack/react-query";
import { getCardByName, getTokenByName } from "@/api/scryfall";

/**
 * Returns a Scryfall image URL for the given card name.
 * If the card already has an imageUrl stored, skips the fetch.
 * Pass isToken=true to search the token database instead of named cards.
 * Pass color (engine format: "W", "WU", "C") to disambiguate same-named tokens.
 */
export function useCardImage(name: string, existingUrl?: string, isToken?: boolean, color?: string) {
  return useQuery({
    queryKey: ["card-image", name, isToken ? "token" : "card", color ?? ""],
    queryFn: async () => {
      const card = isToken ? await getTokenByName(name, color) : await getCardByName(name);
      return card.image_uris?.normal ?? card.image_uris?.large ?? null;
    },
    enabled: !!name && !existingUrl,
    staleTime: Infinity, // card images never change
    gcTime: 1000 * 60 * 60, // keep in cache 1 hour
    retry: false,
  });
}
