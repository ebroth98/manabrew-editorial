import { useCallback, useState } from "react";

export function useDeckSelection() {
  const [selectedCards, setSelectedCards] = useState<Set<string>>(new Set());

  const isSelected = useCallback(
    (cardName: string) => selectedCards.has(cardName.toLowerCase()),
    [selectedCards],
  );

  const toggleCard = useCallback((cardName: string, addToSelection: boolean) => {
    const key = cardName.toLowerCase();
    setSelectedCards((prev) => {
      const next = new Set(addToSelection ? prev : []);
      if (prev.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  }, []);

  const selectCards = useCallback((cardNames: string[], replaceSelection: boolean) => {
    setSelectedCards((prev) => {
      const next = replaceSelection ? new Set<string>() : new Set(prev);
      for (const name of cardNames) {
        next.add(name.toLowerCase());
      }
      return next;
    });
  }, []);

  const clearSelection = useCallback(() => {
    setSelectedCards(new Set());
  }, []);

  const selectAll = useCallback((allCardNames: string[]) => {
    setSelectedCards(new Set(allCardNames.map((n) => n.toLowerCase())));
  }, []);

  return {
    selectedCards,
    isSelected,
    toggleCard,
    selectCards,
    clearSelection,
    selectAll,
  };
}
