import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { searchCards, getRulings, getCardPrints, fetchSets } from "@/api/scryfall";
import type { ScryfallSet } from "@/types/scryfall";

export function useCardSearch(query: string, order?: string, dir?: string) {
  return useInfiniteQuery({
    queryKey: ["cards", "search", query, order, dir],
    queryFn: ({ pageParam = 1 }) => searchCards(query, pageParam as number, order, dir),
    getNextPageParam: (lastPage, allPages) => {
      if (lastPage.has_more) {
        return allPages.length + 1;
      }
      return undefined;
    },
    enabled: query.length > 0,
    initialPageParam: 1,
  });
}

export function useCardRulings(rulingsUri: string | undefined) {
  return useQuery({
    queryKey: ["cards", "rulings", rulingsUri],
    queryFn: () => getRulings(rulingsUri!),
    enabled: !!rulingsUri,
  });
}

export function useCardPrints(printsSearchUri: string | undefined, enabled: boolean = true) {
  return useQuery({
    queryKey: ["cards", "prints", printsSearchUri],
    queryFn: () => getCardPrints(printsSearchUri!),
    enabled: !!printsSearchUri && enabled,
  });
}

/** Fetches all Scryfall sets, cached for 1 hour. */
export function useScryfallSets() {
  return useQuery({
    queryKey: ["scryfall", "sets"],
    queryFn: fetchSets,
    staleTime: 60 * 60 * 1000,
    gcTime: 2 * 60 * 60 * 1000,
  });
}

/** Build a code→name lookup map from the sets query. */
export function useSetLookup(): Map<string, ScryfallSet> {
  const { data } = useScryfallSets();
  if (!data) return new Map();
  return new Map(data.map((s) => [s.code, s]));
}
