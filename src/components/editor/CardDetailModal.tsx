import { useState } from "react";
import { Modal } from "@/components/game/modals/Modal";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { Plus, Loader2, Image as ImageIcon, ChevronDown } from "lucide-react";
import { useCardRulings } from "@/hooks/useCards";
import { usePreferredPrintsStore } from "@/stores/usePreferredPrintsStore";
import { useDeckStore } from "@/stores/useDeckStore";
import { PrintPickerModal } from "@/components/editor/PrintPickerModal";
import { getScryfallImageUrl, getScryfallManaCost } from "@/api/scryfall";
import { scryfallToXMage } from "@/lib/scryfall.utils";
import { useSetLookup } from "@/hooks/useCards";
import { FORMAT_DISPLAY, LEGALITY_STYLES } from "@/lib/constants";
import { toast } from "sonner";
import type { ScryfallCard } from "@/types/scryfall";

interface CardDetailModalProps {
  card: ScryfallCard | null;
  onClose: () => void;
}

export function CardDetailModal({ card: initialCard, onClose }: CardDetailModalProps) {
  const [showPrints, setShowPrints] = useState(false);
  const [showDeckPicker, setShowDeckPicker] = useState(false);
  const [selectedPrint, setSelectedPrint] = useState<ScryfallCard | null>(null);
  const { data: rulingsData, isLoading: rulingsLoading } = useCardRulings(initialCard?.rulings_uri);
  const { setPreferredPrint } = usePreferredPrintsStore();
  const setLookup = useSetLookup();
  const { savedDecks, currentDeck, addToMain, addCardToSavedDeck } = useDeckStore();

  if (!initialCard) return null;

  const card = selectedPrint ?? initialCard;
  const imageUrl = getScryfallImageUrl(card);
  const manaCost = getScryfallManaCost(card);
  const rulings = rulingsData?.data ?? [];

  function handleAddToCurrentDeck() {
    addToMain(scryfallToXMage(card));
    setShowDeckPicker(false);
    toast.success(`Added to ${currentDeck.name}`);
  }

  function handleAddToSavedDeck(deckId: string, deckName: string) {
    addCardToSavedDeck(deckId, scryfallToXMage(card));
    setShowDeckPicker(false);
    toast.success(`Added to ${deckName}`);
  }

  function handleSelectPrint(print: ScryfallCard) {
    setSelectedPrint(print);
    setPreferredPrint(initialCard!.oracle_id, {
      set: print.set,
      collectorNumber: print.collector_number,
      imageUrl: getScryfallImageUrl(print),
    });
  }

  return (
    <>
      <Modal onClose={onClose} maxWidth="max-w-4xl" maxHeight="max-h-[90vh]">
        <Modal.Header onClose={onClose}>
          <div className="flex items-center gap-2">
            <h2 className="text-lg font-bold truncate">{card.name}</h2>
            {manaCost && <ManaSymbols cost={manaCost} size="sm" className="shrink-0" />}
          </div>
        </Modal.Header>

        <Modal.Body className="p-0">
          <ScrollArea className="h-full">
            <div className="p-4 space-y-4">
              <div className="flex gap-6">
                <div className="shrink-0 w-64">
                  {imageUrl ? (
                    <img src={imageUrl} alt={card.name} className="w-full rounded-lg shadow-lg" />
                  ) : (
                    <div className="w-full aspect-[5/7] rounded-lg bg-muted flex items-center justify-center">
                      <span className="text-muted-foreground text-sm">No Image</span>
                    </div>
                  )}
                  <Button
                    variant="outline"
                    size="sm"
                    className="w-full mt-2 gap-1"
                    onClick={() => setShowPrints(true)}
                  >
                    <ImageIcon className="h-3.5 w-3.5" />
                    Show All Printings
                  </Button>
                </div>

                <div className="flex-1 min-w-0 space-y-3">
                  <div>
                    <div className="text-sm font-semibold text-muted-foreground">Type</div>
                    <div className="text-sm">{card.type_line}</div>
                  </div>

                  {card.oracle_text && (
                    <div>
                      <div className="text-sm font-semibold text-muted-foreground">Oracle Text</div>
                      <div className="text-sm whitespace-pre-wrap bg-muted/30 rounded p-2 border">
                        {card.oracle_text}
                      </div>
                    </div>
                  )}

                  {card.power && card.toughness && (
                    <div className="flex gap-4">
                      <div>
                        <span className="text-sm font-semibold text-muted-foreground">P/T: </span>
                        <span className="text-sm font-bold">{card.power}/{card.toughness}</span>
                      </div>
                      <div>
                        <span className="text-sm font-semibold text-muted-foreground">CMC: </span>
                        <span className="text-sm">{card.cmc}</span>
                      </div>
                    </div>
                  )}

                  <div className="flex gap-4 text-sm">
                    <div className="flex items-center gap-1">
                      <span className="font-semibold text-muted-foreground">Set: </span>
                      {setLookup.get(card.set)?.icon_svg_uri && (
                        <img
                          src={setLookup.get(card.set)!.icon_svg_uri}
                          alt=""
                          className="h-4 w-4 shrink-0 brightness-0 dark:invert"
                        />
                      )}
                      <span>{card.set_name} ({card.set.toUpperCase()})</span>
                    </div>
                    <div>
                      <span className="font-semibold text-muted-foreground">Rarity: </span>
                      <span className="capitalize">{card.rarity}</span>
                    </div>
                    <div>
                      <span className="font-semibold text-muted-foreground"># </span>
                      <span>{card.collector_number}</span>
                    </div>
                  </div>

                  <div className="text-sm">
                    <span className="font-semibold text-muted-foreground">Artist: </span>
                    <span>{card.artist}</span>
                  </div>

                  {card.edhrec_rank && (
                    <div className="text-sm">
                      <span className="font-semibold text-muted-foreground">EDHREC Rank: </span>
                      <span>#{card.edhrec_rank.toLocaleString()}</span>
                    </div>
                  )}

                  <div>
                    <div className="text-sm font-semibold text-muted-foreground mb-1">Prices</div>
                    <div className="flex gap-3 text-sm">
                      {card.prices.usd && <span>USD ${card.prices.usd}</span>}
                      {card.prices.usd_foil && <span>Foil ${card.prices.usd_foil}</span>}
                      {card.prices.eur && <span>EUR €{card.prices.eur}</span>}
                      {card.prices.tix && <span>TIX {card.prices.tix}</span>}
                      {!card.prices.usd && !card.prices.eur && !card.prices.tix && (
                        <span className="text-muted-foreground">No price data</span>
                      )}
                    </div>
                  </div>
                </div>
              </div>

              <div>
                <div className="text-sm font-semibold text-muted-foreground mb-1">Legalities</div>
                <div className="grid grid-cols-3 gap-1.5">
                  {Object.entries(FORMAT_DISPLAY).map(([key, label]) => {
                    const status = card.legalities[key] ?? "not_legal";
                    return (
                      <Badge
                        key={key}
                        variant="outline"
                        className={cn("text-xs justify-between px-2 py-0.5", LEGALITY_STYLES[status])}
                      >
                        <span>{label}</span>
                        <span className="capitalize">{status.replace("_", " ")}</span>
                      </Badge>
                    );
                  })}
                </div>
              </div>

              <div>
                <div className="text-sm font-semibold text-muted-foreground mb-1">
                  Rulings {rulings.length > 0 && `(${rulings.length})`}
                </div>
                {rulingsLoading && (
                  <div className="flex justify-center py-4">
                    <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                  </div>
                )}
                {!rulingsLoading && rulings.length === 0 && (
                  <p className="text-sm text-muted-foreground">No rulings available.</p>
                )}
                {rulings.length > 0 && (
                  <div className="space-y-1.5 max-h-48 overflow-y-auto">
                    {rulings.map((r, i) => (
                      <div key={i} className="text-xs border rounded p-2 bg-muted/20">
                        <div className="text-muted-foreground mb-0.5">
                          {r.published_at} — {r.source}
                        </div>
                        <div>{r.comment}</div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </ScrollArea>
        </Modal.Body>

        <Modal.Footer>
          <div className="flex gap-2 w-full justify-between">
            <div className="relative">
              <Button size="sm" className="gap-1" onClick={() => setShowDeckPicker((v) => !v)}>
                <Plus className="h-3.5 w-3.5" />
                Add to Deck
                <ChevronDown className="h-3 w-3 ml-1" />
              </Button>

              {showDeckPicker && (
                <div className="absolute bottom-full left-0 mb-1 w-64 bg-popover border rounded-md shadow-lg py-1 z-10">
                  <div className="px-2 py-1 text-xs font-semibold text-muted-foreground">
                    Select a deck
                  </div>
                  {currentDeck.name && (
                    <button
                      type="button"
                      className="w-full text-left px-3 py-1.5 text-sm hover:bg-muted flex items-center gap-2"
                      onClick={handleAddToCurrentDeck}
                    >
                      <span className="flex-1 truncate">{currentDeck.name}</span>
                      <Badge variant="outline" className="text-[10px] px-1 py-0 shrink-0">
                        editing
                      </Badge>
                    </button>
                  )}
                  {savedDecks.length > 0 && <div className="border-t my-1" />}
                  <ScrollArea className={savedDecks.length > 6 ? "max-h-48" : ""}>
                    {savedDecks.map((s) => (
                      <button
                        key={s.id}
                        type="button"
                        className="w-full text-left px-3 py-1.5 text-sm hover:bg-muted flex items-center gap-2"
                        onClick={() => handleAddToSavedDeck(s.id, s.deck.name)}
                      >
                        <span className="flex-1 truncate">{s.deck.name}</span>
                        <span className="text-xs text-muted-foreground shrink-0">
                          {s.deck.cards.length} cards
                        </span>
                      </button>
                    ))}
                  </ScrollArea>
                  {savedDecks.length === 0 && !currentDeck.name && (
                    <div className="px-3 py-2 text-sm text-muted-foreground">No decks available</div>
                  )}
                </div>
              )}
            </div>
            <Button size="sm" variant="ghost" onClick={onClose}>Close</Button>
          </div>
        </Modal.Footer>
      </Modal>

      {showPrints && (
        <PrintPickerModal
          cardName={card.name}
          onClose={() => setShowPrints(false)}
          onSelect={handleSelectPrint}
        />
      )}
    </>
  );
}
