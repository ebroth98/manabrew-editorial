import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { DeckLabelBadge } from "@/components/deck/DeckLabelBadge";
import { FormatBadge } from "@/components/game/FormatBadge";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { Pencil, Trash2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { extractColors } from "@/views/myDecks.utils";
import type { SavedDeck } from "@/stores/useDeckStore";
import type { Deck, Card } from "@/types/openmagic";
import { getCardByName } from "@/api/scryfall";

/** Returns the card object used as cover (searches main + commanders). */
function resolveCoverCard(deck: Deck): Card | undefined {
  const allCards = [...deck.cards, ...(deck.commanders ?? [])];
  if (deck.coverCardName) {
    const found = allCards.find((c) => c.name === deck.coverCardName);
    if (found) return found;
  }
  return deck.commanders?.[0] ?? deck.cards[0];
}

/** Scryfall art-crop URL for a card name (front face, direct redirect). */
function frontArtCropUrl(cardName: string): string {
  return `https://api.scryfall.com/cards/named?exact=${encodeURIComponent(cardName)}&format=image&version=art_crop`;
}

interface DeckGridCardProps {
  deck: SavedDeck;
  onOpen: () => void;
  onDelete: () => void;
  onRename: () => void;
}

export function DeckGridCard({ deck, onOpen, onDelete, onRename }: DeckGridCardProps) {
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [artError, setArtError] = useState(false);
  const colors = extractColors(deck.deck.cards);
  const colorCost = colors.map((c) => `{${c}}`).join("");

  const coverCard = resolveCoverCard(deck.deck);
  const wantBackFace = coverCard?.isDoubleFaced && deck.deck.coverCardFace === 1;

  // Fetch back-face art_crop only when a DFC is set to show its back
  const { data: backFaceArtUrl } = useQuery({
    queryKey: ["cover-back-face-art", coverCard?.name],
    queryFn: async () => {
      const scryfall = await getCardByName(coverCard!.name);
      return scryfall.card_faces?.[1]?.image_uris?.art_crop ?? null;
    },
    enabled: !!coverCard && wantBackFace,
    staleTime: Infinity,
    gcTime: 1000 * 60 * 60,
    retry: false,
  });

  const artUrl = !coverCard
    ? undefined
    : wantBackFace
      ? (backFaceArtUrl ?? undefined)
      : frontArtCropUrl(coverCard.name);

  return (
    <>
      <div
        className={cn(
          "relative group cursor-pointer rounded-lg overflow-hidden border bg-muted",
          "aspect-[4/3] transition-all hover:ring-2 hover:ring-primary hover:border-primary",
        )}
        onClick={onOpen}
      >
        {/* Cover art */}
        {artUrl && !artError ? (
          <img
            src={artUrl}
            alt={coverCard?.name}
            className="absolute inset-0 w-full h-full object-cover"
            onError={() => setArtError(true)}
          />
        ) : (
          <div className="absolute inset-0 bg-gradient-to-br from-muted-foreground/5 to-muted-foreground/20" />
        )}

        {/* Darkening overlay so bottom info is always readable */}
        <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-black/10" />

        {/* Action buttons — visible on hover */}
        <div className="absolute top-1.5 right-1.5 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity z-10">
          <Button
            size="icon"
            variant="secondary"
            className="h-6 w-6 bg-background/80 backdrop-blur-sm hover:bg-background"
            title="Rename"
            onClick={(e) => { e.stopPropagation(); onRename(); }}
          >
            <Pencil className="h-3 w-3" />
          </Button>
          <Button
            size="icon"
            variant="secondary"
            className="h-6 w-6 bg-background/80 backdrop-blur-sm hover:bg-background text-destructive hover:text-destructive"
            title="Delete"
            onClick={(e) => { e.stopPropagation(); setConfirmDelete(true); }}
          >
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>

        {/* Bottom info overlay */}
        <div className="absolute bottom-0 left-0 right-0 px-2 pt-6 pb-2 z-10">
          <p className="text-white text-xs font-semibold truncate leading-tight drop-shadow">
            {deck.deck.name}
          </p>
          <div className="flex items-center gap-1 mt-1 flex-wrap">
            <FormatBadge formatId={deck.deck.format ?? "constructed"} />
            {colorCost && <ManaSymbols cost={colorCost} size="sm" />}
            {deck.deck.labels?.map((label) => (
              <DeckLabelBadge key={label.name} label={label} size="sm" />
            ))}
          </div>
        </div>
      </div>

      <Dialog open={confirmDelete} onOpenChange={setConfirmDelete}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Delete Deck</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete &ldquo;{deck.deck.name}&rdquo;? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="gap-2">
            <Button variant="outline" size="sm" onClick={() => setConfirmDelete(false)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => { setConfirmDelete(false); onDelete(); }}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
