/**
 * Hook that extracts token production data from deck cards.
 * Fetches Scryfall card data (with caching) and reads the `all_parts`
 * field to find which tokens each card can create.
 */

import { useMemo } from "react";
import type { Card, DeckToken } from "@/types/manabrew";
import { useScryfallStore } from "@/stores/useScryfallStore";
import { useQuery } from "@tanstack/react-query";

// Re-export so consumers don't need to import from manabrew
export type { DeckToken };

function scryfallIdFromUri(uri: string): string | null {
  return uri.split("/").filter(Boolean).pop() ?? null;
}

/**
 * Extract tokens produced by the given deck cards.
 * Uses Scryfall's `all_parts` field to find token relationships.
 *
 * @param deckCards All cards in the deck (main + commanders + sideboard etc.)
 * @param cached Previously persisted tokens — returned while fresh fetch is in flight.
 * @returns Array of DeckToken sorted by name, plus loading state.
 */
export function useTokenProducers(
  deckCards: Card[],
  cached?: DeckToken[],
): {
  tokens: DeckToken[];
  isLoading: boolean;
} {
  const uniqueNames = useMemo(() => {
    const names = new Set<string>();
    for (const c of deckCards) names.add(c.name);
    return [...names].sort();
  }, [deckCards]);

  const queryKey = useMemo(() => uniqueNames.join("\0"), [uniqueNames]);

  const { data: scryfallMap, isLoading } = useQuery({
    queryKey: ["token-producers", queryKey],
    queryFn: async () => {
      const scryfall = useScryfallStore.getState();
      const tokenMap = new Map<
        string,
        { name: string; typeLine: string; producers: Set<string>; tokenId?: string }
      >();

      const producerResults = await Promise.allSettled(
        uniqueNames.map((name) => scryfall.getCard({ name })),
      );

      for (const result of producerResults) {
        if (result.status !== "fulfilled") continue;
        const producer = result.value.info;
        if (!producer.all_parts) continue;

        for (const part of producer.all_parts) {
          if (part.component !== "token") continue;
          const existing = tokenMap.get(part.name);
          if (existing) {
            existing.producers.add(producer.name);
            existing.tokenId ??= scryfallIdFromUri(part.uri) ?? undefined;
          } else {
            tokenMap.set(part.name, {
              name: part.name,
              typeLine: part.type_line,
              producers: new Set([producer.name]),
              tokenId: scryfallIdFromUri(part.uri) ?? undefined,
            });
          }
        }
      }

      const tokens = await Promise.all(
        [...tokenMap.values()].map(async (token): Promise<DeckToken> => {
          if (!token.tokenId) {
            return {
              name: token.name,
              typeLine: token.typeLine,
              producers: [...token.producers].sort(),
            };
          }

          try {
            const tokenCard = await scryfall.getCard({ id: token.tokenId, name: token.name });
            return {
              name: token.name,
              typeLine: token.typeLine,
              producers: [...token.producers].sort(),
              setCode: tokenCard.info.set,
              cardNumber: tokenCard.info.collector_number,
              imageUrl: tokenCard.uris.normal ?? tokenCard.uris.large,
            };
          } catch {
            return {
              name: token.name,
              typeLine: token.typeLine,
              producers: [...token.producers].sort(),
            };
          }
        }),
      );

      return tokens.sort((a, b) => a.name.localeCompare(b.name));
    },
    enabled: uniqueNames.length > 0,
    staleTime: 1000 * 60 * 30, // 30 min — token relationships don't change
    gcTime: 1000 * 60 * 60,
    retry: false,
  });

  // Fall back to cached tokens while the fetch is in flight, but keep the
  // memoized computation independent of `cached` to avoid feedback loops with the store.
  const tokens = scryfallMap ?? cached ?? [];

  return { tokens, isLoading };
}
