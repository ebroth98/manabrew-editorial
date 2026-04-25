import { useState } from "react";
import { cn } from "@/lib/utils";
import { useDeckCoverUrl, type DeckCoverSource } from "./deckCover.utils";

interface DeckCoverImageProps {
  cover?: DeckCoverSource;
  alt?: string;
  className?: string;
  fallbackClassName?: string;
}

export function DeckCoverImage({ cover, alt, className, fallbackClassName }: DeckCoverImageProps) {
  const [artError, setArtError] = useState(false);
  const artUrl = useDeckCoverUrl(cover);

  const [prevArtUrl, setPrevArtUrl] = useState(artUrl);
  if (prevArtUrl !== artUrl) {
    setPrevArtUrl(artUrl);
    setArtError(false);
  }

  if (artUrl && !artError) {
    return (
      <img
        src={artUrl}
        alt={alt ?? cover?.cardName ?? "Deck cover"}
        loading="lazy"
        className={cn(
          "absolute inset-0 h-full w-full object-cover",
          "transition-transform duration-300 ease-out group-hover:scale-110",
          className,
        )}
        onError={() => setArtError(true)}
      />
    );
  }

  return (
    <div
      className={cn(
        "absolute inset-0 bg-gradient-to-br from-muted-foreground/5 to-muted-foreground/20",
        fallbackClassName,
      )}
    />
  );
}
