import type { Card as CardType } from "@/types/xmage";
import { cn } from "@/lib/utils";
import { useState } from "react";

interface CardProps {
  card: CardType;
  className?: string;
  isTapped?: boolean;
  onClick?: () => void;
}

export function Card({ card, className, isTapped, onClick }: CardProps) {
  const [hasError, setHasError] = useState(false);

  return (
    <div
      className={cn(
        "relative rounded-lg border bg-card text-card-foreground shadow-sm transition-transform hover:scale-105 cursor-pointer overflow-hidden",
        "w-[150px] aspect-[5/7]",
        isTapped && "rotate-90",
        className
      )}
      title={card.name}
      onClick={onClick}
    >
      {card.imageUrl && !hasError ? (
        <img
          src={card.imageUrl}
          alt={card.name}
          className="absolute inset-0 w-full h-full object-contain rounded-lg"
          onError={() => setHasError(true)}
          loading="lazy"
        />
      ) : (
        <div className="absolute inset-0 p-2 flex flex-col justify-between">
          <div className="flex justify-between items-start gap-1">
            <span className="font-bold text-xs leading-tight line-clamp-2">{card.name}</span>
            <span className="text-xs font-mono shrink-0">{card.manaCost}</span>
          </div>
          <div className="flex-1 flex items-center justify-center px-1">
            <span className="text-xs text-muted-foreground text-center line-clamp-5">
              {card.text}
            </span>
          </div>
          <div className="flex justify-between items-end">
            <span className="text-xs text-muted-foreground truncate">
              {card.types?.join(" ")}
            </span>
            {card.power && card.toughness && (
              <span className="font-bold text-sm shrink-0">{card.power}/{card.toughness}</span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
