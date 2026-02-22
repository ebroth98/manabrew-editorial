import { createPortal } from "react-dom";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { Card as CardType, StackObject } from "@/types/xmage";
import { cn } from "@/lib/utils";
import { useState } from "react";

interface SpellStackModalProps {
  stack: StackObject[];
  /** Stack entry IDs the player may target (counter). Empty means view-only. */
  validSpellIds: string[];
  onTarget: (spellId: string) => void;
  onCancel: () => void;
}

function stackObjectToCardStub(obj: StackObject): CardType {
  return {
    id: obj.sourceId,
    name: obj.name,
    setCode: "",
    cardNumber: "",
    color: "",
    manaCost: "",
    types: [],
    subtypes: [],
    supertypes: [],
    text: obj.text,
    isPlayable: false,
    isSelected: false,
    isChoosable: false,
    controllerId: "",
    ownerId: "",
    zoneId: "",
  };
}

export function SpellStackModal({ stack, validSpellIds, onTarget, onCancel }: SpellStackModalProps) {
  const [hoveredCard, setHoveredCard] = useState<CardType | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });

  const isTargeting = validSpellIds.length > 0;

  // Display newest (top of stack) first — stack[last] = top, stack[0] = bottom
  const displayStack = [...stack].reverse();

  return createPortal(
    <div
      className="fixed inset-0 z-[9000] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onCancel}
    >
      <div
        className="bg-card border rounded-xl shadow-2xl flex flex-col max-w-3xl w-full max-h-[85vh] mx-4 animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <div>
            <h2 className="font-semibold text-base">
              {isTargeting ? "Choose a Spell to Counter" : "Spells on the Stack"}
            </h2>
            <p className="text-xs text-muted-foreground">
              {stack.length} spell{stack.length !== 1 ? "s" : ""} on the stack
              {" · "}
              Top of stack is shown first
            </p>
          </div>
          {isTargeting && (
            <Badge variant="secondary">{validSpellIds.length} targetable</Badge>
          )}
        </div>

        {/* Instructions (targeting only) */}
        {isTargeting && (
          <div className="px-4 py-2 bg-blue-50 dark:bg-blue-950/20 border-b">
            <p className="text-sm font-semibold text-blue-700 dark:text-blue-400 text-center">
              Click a highlighted spell to counter it.
            </p>
          </div>
        )}

        {/* Card row — top of stack first (left), bottom last (right) */}
        <div className="overflow-y-auto p-4 flex-1">
          {stack.length === 0 ? (
            <p className="text-sm text-muted-foreground italic text-center py-8">
              The stack is empty.
            </p>
          ) : (
            <div className="flex flex-wrap gap-6 content-start justify-center">
              {displayStack.map((obj, idx) => {
                const isValid = validSpellIds.includes(obj.id);
                const cardStub = stackObjectToCardStub(obj);
                const isTop = idx === 0;
                return (
                  <div
                    key={obj.id}
                    className={cn(
                      "shrink-0 flex flex-col items-center gap-1 group",
                      isValid ? "cursor-pointer" : "cursor-default",
                      !isValid && isTargeting && "opacity-50",
                    )}
                    onMouseEnter={(e) => {
                      setHoveredCard(cardStub);
                      setMousePos({ x: e.clientX, y: e.clientY });
                    }}
                    onMouseLeave={() => setHoveredCard(null)}
                    onClick={isValid ? () => onTarget(obj.id) : undefined}
                  >
                    <Card
                      card={cardStub}
                      className={cn(
                        "w-[100px] h-[140px] transition-transform",
                        isValid && "ring-2 ring-blue-400 group-hover:scale-105 group-hover:-translate-y-2",
                      )}
                    />
                    <div className="flex items-center gap-1">
                      <Badge
                        variant={isTop ? "default" : "outline"}
                        className="text-[10px] h-4 px-1"
                      >
                        {isTop ? "TOP" : `+${idx}`}
                      </Badge>
                      {isValid && (
                        <Badge variant="secondary" className="text-[10px] h-4 px-1 text-blue-600">
                          ← Counter
                        </Badge>
                      )}
                    </div>
                    {/* Ability text below the card */}
                    {obj.text && (
                      <p className="text-[10px] text-muted-foreground text-center max-w-[100px] line-clamp-3 leading-tight">
                        {obj.text}
                      </p>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end px-4 py-3 border-t">
          <Button variant="outline" size="sm" onClick={onCancel}>
            {isTargeting ? "Cancel" : "Close"}
          </Button>
        </div>
      </div>

      {hoveredCard && (
        <CardPreview
          card={hoveredCard}
          mouseX={mousePos.x}
          mouseY={mousePos.y}
        />
      )}
    </div>,
    document.body,
  );
}
