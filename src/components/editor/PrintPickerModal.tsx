import { useState, useEffect } from "react";
import { Modal } from "@/components/game/modals/Modal";
import { Loader2 } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useDeckStore } from "@/stores/useDeckStore";
import { getCardByName, getCardPrints, getTokenByName } from "@/api/scryfall";
import { useSetLookup } from "@/hooks/useCards";
import type { ScryfallCard } from "@/types/scryfall";

interface PrintPickerModalProps {
  cardName: string | null;
  onClose: () => void;
  onSelect?: (print: ScryfallCard) => void;
  isToken?: boolean;
}

export function PrintPickerModal({ cardName, onClose, onSelect, isToken }: PrintPickerModalProps) {
  const [prints, setPrints] = useState<ScryfallCard[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { updatePrint } = useDeckStore();
  const setLookup = useSetLookup();

  useEffect(() => {
    if (!cardName) {
      setPrints([]);
      return;
    }

    let mounted = true;

    async function fetchPrints() {
      setIsLoading(true);
      setError(null);
      try {
        if (isToken) {
          // Find one token, then fetch ALL its printings via prints_search_uri
          const tokenCard = await getTokenByName(cardName!);
          if (tokenCard?.prints_search_uri) {
            const res = await getCardPrints(tokenCard.prints_search_uri);
            if (mounted) setPrints(res.data || []);
          } else if (tokenCard && mounted) {
            setPrints([tokenCard]);
          } else if (mounted) {
            setPrints([]);
          }
        } else {
          const baseCard = await getCardByName(cardName!);
          if (baseCard.prints_search_uri) {
            const res = await getCardPrints(baseCard.prints_search_uri);
            if (mounted) setPrints(res.data || []);
          } else if (mounted) {
            setPrints([baseCard]);
          }
        }
      } catch {
        if (mounted) {
          setError("Failed to fetch printings.");
        }
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    }

    fetchPrints();
    return () => {
      mounted = false;
    };
  }, [cardName, isToken]);

  if (!cardName) return null;

  return (
    <Modal
      onClose={onClose}
      maxWidth="max-w-4xl"
      maxHeight="max-h-[80vh]"
      backdropClassName="z-[9100]"
    >
      <Modal.Header onClose={onClose}>
        <h2 className="text-lg font-bold">Select Printing: {cardName}</h2>
      </Modal.Header>

      <Modal.Body>
        {isLoading && (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        )}
        {error && (
          <div className="flex items-center justify-center py-12 text-destructive">{error}</div>
        )}

        {!isLoading && !error && (
          <ScrollArea className="h-full">
            <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-4">
              {prints.map((p) => {
                const face = p.card_faces
                  ? p.card_faces.find((f) => f.name.toLowerCase() === cardName.toLowerCase()) ||
                    p.card_faces[0]
                  : null;
                const imageUrl =
                  face?.image_uris?.normal ||
                  face?.image_uris?.large ||
                  p.image_uris?.normal ||
                  p.image_uris?.large;

                return (
                  <div
                    key={p.id}
                    className="group cursor-pointer flex flex-col gap-1 items-center"
                    onClick={() => {
                      if (onSelect) {
                        onSelect(p);
                      } else {
                        updatePrint(cardName, p);
                      }
                      onClose();
                    }}
                  >
                    <div className="w-full aspect-[5/7] rounded-[4%] overflow-hidden border-2 border-transparent group-hover:border-primary transition-colors bg-muted flex items-center justify-center relative">
                      {imageUrl ? (
                        <img
                          src={imageUrl}
                          alt={`${p.set_name} printing`}
                          className="w-full h-full object-contain"
                          loading="lazy"
                        />
                      ) : (
                        <span className="text-xs text-muted-foreground text-center">No Image</span>
                      )}
                    </div>
                    <div className="text-center w-full">
                      <div
                        className="text-xs font-semibold truncate flex items-center justify-center gap-1"
                        title={p.set_name}
                      >
                        {setLookup.get(p.set)?.icon_svg_uri && (
                          <img
                            src={setLookup.get(p.set)!.icon_svg_uri}
                            alt=""
                            className="h-3.5 w-3.5 shrink-0 brightness-0 dark:invert"
                          />
                        )}
                        <span className="truncate">{p.set_name}</span>
                      </div>
                      <div className="text-[10px] text-muted-foreground uppercase flex gap-1 justify-center">
                        <span>{p.set}</span>
                        <span>•</span>
                        <span>#{p.collector_number}</span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </ScrollArea>
        )}
      </Modal.Body>
    </Modal>
  );
}
