import { useState, useRef, useEffect } from "react";
import { useCardSearch } from "@/hooks/useCards";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/game/Card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useDeckStore } from "@/stores/useDeckStore";
import { Loader2 } from "lucide-react";
import type { ScryfallCard } from "@/types/scryfall";
import type { Card as XMageCard } from "@/types/xmage";

export function CardSearch() {
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const { addToMain, addToSide } = useDeckStore();
  const observerTarget = useRef(null);

  const {
    data,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    status,
  } = useCardSearch(debouncedQuery);

  // Debounce query
  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedQuery(query);
    }, 500);
    return () => clearTimeout(handler);
  }, [query]);

  // Infinite Scroll
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasNextPage) {
          fetchNextPage();
        }
      },
      { threshold: 1.0 }
    );

    if (observerTarget.current) {
      observer.observe(observerTarget.current);
    }

    return () => observer.disconnect();
  }, [hasNextPage, fetchNextPage]);

  const mapScryfallToXMage = (sfCard: ScryfallCard): XMageCard => ({
    id: sfCard.id,
    name: sfCard.name,
    setCode: sfCard.set,
    cardNumber: sfCard.collector_number,
    color: sfCard.colors ? sfCard.colors.join("") : "",
    manaCost: sfCard.mana_cost || "",
    cmc: sfCard.cmc,
    types: sfCard.type_line.split("—")[0].trim().split(" "),
    subtypes: sfCard.type_line.split("—")[1]?.trim().split(" ") || [],
    supertypes: [],
    power: sfCard.power,
    toughness: sfCard.toughness,
    text: sfCard.oracle_text || "",
    imageUrl: sfCard.image_uris?.normal,
    isPlayable: true,
    isSelected: false,
    isChoosable: true,
    controllerId: "",
    ownerId: "",
    zoneId: "",
  });

  return (
    <div className="flex flex-col h-full w-full">
      <div className="p-4 border-b flex gap-2">
        <Input
          placeholder="Search cards (e.g. 'Jace', 'Counterspell', 't:creature c:u')"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="max-w-md"
        />
        <Button variant="outline" disabled={!query}>
          Search
        </Button>
      </div>

      <ScrollArea className="flex-1 p-4">
        {status === "pending" && debouncedQuery && (
          <div className="flex justify-center p-8">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        )}
        
        {status === "error" && (
          <div className="text-center p-8 text-red-500">
            Error fetching cards. Please try again.
          </div>
        )}

        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8 gap-4 pb-4">
          {data?.pages.map((group) =>
            group.data.map((card) => (
              <div key={card.id} className="relative group p-2">
                <Card 
                  card={mapScryfallToXMage(card)} 
                  className="w-full h-auto aspect-[5/7]"
                />
                <div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-2 rounded-lg">
                  <Button 
                    size="sm" 
                    variant="secondary"
                    onClick={() => addToMain(mapScryfallToXMage(card))}
                  >
                    Main
                  </Button>
                  <Button 
                    size="sm" 
                    variant="secondary"
                    onClick={() => addToSide(mapScryfallToXMage(card))}
                  >
                    Side
                  </Button>
                </div>
              </div>
            ))
          )}
        </div>
        
        <div ref={observerTarget} className="h-10 flex justify-center items-center">
          {isFetchingNextPage && <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />}
        </div>
      </ScrollArea>
    </div>
  );
}
