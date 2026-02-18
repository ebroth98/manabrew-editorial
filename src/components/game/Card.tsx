import type { Card as CardType } from "@/types/xmage";
import { cn } from "@/lib/utils";
import { useState } from "react";

interface CardProps {
  card: CardType;
  className?: string;
  isTapped?: boolean;
}

export function Card({ card, className, isTapped }: CardProps) {
  const [hasError, setHasError] = useState(false);

  return (
    <div 
      className={cn(
        "relative w-[150px] h-[210px] rounded-lg border bg-card text-card-foreground shadow-sm transition-transform hover:scale-105 cursor-pointer",
        isTapped && "rotate-90",
        className
      )}
      title={card.name}
    >
      {card.imageUrl && !hasError ? (
        <img 
          src={card.imageUrl} 
          alt={card.name} 
          className="w-full h-full object-cover rounded-lg"
          onError={() => setHasError(true)}
        />
      ) : (
        <div className="p-2 flex flex-col h-full justify-between">
          <div className="flex justify-between items-start">
            <span className="font-bold text-sm leading-tight">{card.name}</span>
            <span className="text-xs font-mono">{card.manaCost}</span>
          </div>
          <div className="flex-1 flex items-center justify-center">
            <span className="text-xs text-muted-foreground text-center line-clamp-6">
              {card.text}
            </span>
          </div>
          <div className="flex justify-end font-bold text-sm">
            {card.power && card.toughness ? `${card.power}/${card.toughness}` : ''}
          </div>
        </div>
      )}
    </div>
  );
}
