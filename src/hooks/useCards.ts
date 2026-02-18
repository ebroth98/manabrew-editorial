import { useInfiniteQuery } from "@tanstack/react-query";
import { searchCards } from "@/api/scryfall";

export function useCardSearch(query: string) {
  return useInfiniteQuery({
    queryKey: ['cards', 'search', query],
    queryFn: ({ pageParam = 1 }) => searchCards(query, pageParam as number),
    getNextPageParam: (lastPage, allPages) => {
      if (lastPage.has_more) {
        return allPages.length + 1;
      }
      return undefined;
    },
    enabled: query.length > 2, // Only fetch if query is meaningful
    initialPageParam: 1,
  });
}
