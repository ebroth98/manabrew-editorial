import { useState, useEffect } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Loader2 } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useDeckStore } from "@/stores/useDeckStore";
import { getCardByName, getCardPrints } from "@/api/scryfall";
import type { ScryfallCard } from "@/types/scryfall";

interface PrintPickerModalProps {
  cardName: string | null;
  onClose: () => void;
}

export function PrintPickerModal({ cardName, onClose }: PrintPickerModalProps) {
  const [prints, setPrints] = useState<ScryfallCard[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { updatePrint } = useDeckStore();

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
        const baseCard = await getCardByName(cardName!);
        if (baseCard.prints_search_uri) {
          const res = await getCardPrints(baseCard.prints_search_uri);
          if (mounted) {
            setPrints(res.data || []);
          }
        } else if (mounted) {
          setPrints([baseCard]);
        }
      } catch (err) {
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
  }, [cardName]);

  if (!cardName) return null;

  return (
    <Dialog open={!!cardName} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-4xl h-[80vh] flex flex-col p-4">
        <DialogHeader>
          <DialogTitle>Select Printing: {cardName}</DialogTitle>
        </DialogHeader>

        <div className="flex-1 min-h-0 relative">
          {isLoading && (
            <div className="absolute inset-0 flex items-center justify-center">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          )}
          {error && (
            <div className="absolute inset-0 flex items-center justify-center text-red-500">
              {error}
            </div>
          )}

          {!isLoading && !error && (
            <ScrollArea className="h-full">
              <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-4 p-4">
                {prints.map((p) => {
                  const face = p.card_faces
                    ? p.card_faces.find((f) => f.name.toLowerCase() === cardName.toLowerCase()) || p.card_faces[0]
                    : null;
                  const imageUrl = face?.image_uris?.normal || face?.image_uris?.large || p.image_uris?.normal || p.image_uris?.large;

                  return (
                    <div
                      key={p.id}
                      className="group cursor-pointer flex flex-col gap-1 items-center"
                      onClick={() => {
                        updatePrint(cardName, p);
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
                        <div className="text-xs font-semibold truncate" title={p.set_name}>
                          {p.set_name}
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
        </div>
      </DialogContent>
    </Dialog>
  );
}
