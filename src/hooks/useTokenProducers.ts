/**
 * Hook that extracts token production data from deck cards.
 * Fetches Scryfall card data (with caching) and reads the `all_parts`
 * field to find which tokens each card can create.
 */

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { fetchCardCollection } from "@/api/scryfall";
import type { ScryfallCard } from "@/types/scryfall";
import type { Card, DeckToken } from "@/types/openmagic";

// Re-export so consumers don't need to import from openmagic
export type { DeckToken };

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
      if (uniqueNames.length === 0) return new Map<string, ScryfallCard>();
      return fetchCardCollection(uniqueNames.map((n) => ({ name: n })));
    },
    enabled: uniqueNames.length > 0,
    staleTime: 1000 * 60 * 30, // 30 min — token relationships don't change
    gcTime: 1000 * 60 * 60,
    retry: false,
  });

  const computedTokens = useMemo(() => {
    if (!scryfallMap || scryfallMap.size === 0) return null;

    const tokenMap = new Map<string, { name: string; typeLine: string; producers: Set<string> }>();

    for (const [, sc] of scryfallMap) {
      if (!sc.all_parts) continue;
      for (const part of sc.all_parts) {
        if (part.component !== "token") continue;
        const existing = tokenMap.get(part.name);
        if (existing) {
          existing.producers.add(sc.name);
        } else {
          tokenMap.set(part.name, {
            name: part.name,
            typeLine: part.type_line,
            producers: new Set([sc.name]),
          });
        }
      }
    }

    // Filter: only show tokens whose producers are actually in the deck.
    // Do NOT merge print data here — that's the store's job.
    const deckNameSet = new Set(uniqueNames.map((n) => n.toLowerCase()));
    return [...tokenMap.values()]
      .filter((t) => [...t.producers].some((p) => deckNameSet.has(p.toLowerCase())))
      .map(
        (t): DeckToken => ({
          name: t.name,
          typeLine: t.typeLine,
          producers: [...t.producers].sort(),
        }),
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [scryfallMap, uniqueNames]);

  // Fall back to cached tokens while the fetch is in flight, but keep the
  // memoized computation independent of `cached` to avoid feedback loops with the store.
  const tokens = computedTokens ?? cached ?? [];

  return { tokens, isLoading };
}
