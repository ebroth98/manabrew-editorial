import { useState, useCallback } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import {
  X, Minus, Plus, Download, Upload, Crown,
  Tag, Move, MousePointer2,
} from "lucide-react";
import { useDraggable, useDroppable } from "@dnd-kit/core";
import { cn } from "@/lib/utils";
import type { Card } from "@/types/openmagic";
import type { CardGroup, ViewMode, SectionDefinition } from "./deckBuilder.utils";
import { CARD_WIDTH_MAP, GRID_COLS, getTaggedGroups } from "./deckBuilder.utils";
import { CARD_RING } from "@/components/game/game.styles";
import { DROP_ZONE } from "@/lib/constants";
import { useMarquee } from "@/hooks/useMarqueeSelection";
import {
  CardCountBadge, CardThumbnail, CardHoverOverlay, CollapsibleHeader, EmptyDropZone,
  buildCardActions, handleCardClick,
} from "./deckEditor.primitives";

// ─── Draggable Stack Card ─────────────────────────────────────────────────────

function DraggableStackCard({
  group, dragId, cardWidth, cardHeight, index,
  onAddOne, onRemoveOne, onHover, onLeave, onUntag,
  isSelected, onSelect, onShowInfo,
  topOffset, onCardHover, onCardLeave,
}: {
  group: CardGroup;
  dragId: string;
  cardWidth: number;
  cardHeight: number;
  index: number;
  onAddOne: () => void;
  onRemoveOne: () => void;
  onHover: (card: Card, x: number, y: number) => void;
  onLeave: () => void;
  onUntag?: (cardName: string) => void;
  isSelected?: boolean;
  onSelect?: (cardName: string, addToSelection: boolean) => void;
  onShowInfo?: () => void;
  topOffset: number;
  onCardHover: (index: number) => void;
  onCardLeave: () => void;
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: dragId,
    data: { type: "deck-card", card: group.card, name: group.card.name },
  });

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={cn(
        "absolute left-0 group cursor-grab active:cursor-grabbing transition-[top] duration-200 ease-out",
        isDragging && "opacity-30",
        isSelected && cn(CARD_RING.selected, "z-50"),
      )}
      style={{ top: topOffset, width: cardWidth, zIndex: index + 1 }}
      data-card-name={group.card.name}
      onMouseEnter={(e) => { onCardHover(index); onHover(group.card, e.clientX, e.clientY); }}
      onMouseMove={(e) => onHover(group.card, e.clientX, e.clientY)}
      onMouseLeave={() => { onCardLeave(); onLeave(); }}
      onClick={(e) => handleCardClick(e, group.card.name, onSelect, onShowInfo)}
    >
      <CardThumbnail
        imageUrl={group.card.imageUrl}
        name={group.card.name}
        className="block rounded-[4%] border-border/40"
        fallbackClassName="rounded-[4%]"
        fallbackStyle={{ width: cardWidth, height: cardHeight }}
      />
      <CardCountBadge count={group.count} className="border-white/30 shadow" />
      <CardHoverOverlay
        actions={buildCardActions(onAddOne, onRemoveOne, onUntag ? () => onUntag(group.card.name) : undefined)}
        rounded="rounded-[4%]"
      />
    </div>
  );
}

// ─── Stack Column Component ───────────────────────────────────────────────────

interface StackColumnProps {
  label: string;
  sectionId: string;
  groups: CardGroup[];
  cardWidth: number;
  onAddOne: (g: CardGroup) => void;
  onRemoveOne: (name: string) => void;
  onHover: (card: Card, x: number, y: number) => void;
  onLeave: () => void;
  onUntag?: (cardName: string) => void;
  selectedCards?: Set<string>;
  onSelectCard?: (cardName: string, addToSelection: boolean) => void;
  onShowInfo?: (cardName: string) => void;
}

function StackColumn({
  label, sectionId, groups, cardWidth,
  onAddOne, onRemoveOne, onHover, onLeave, onUntag,
  selectedCards, onSelectCard, onShowInfo,
}: StackColumnProps) {
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);
  const cardHeight = Math.round(cardWidth * 1.4);
  const peek = Math.round(cardHeight * 0.22);
  const count = groups.reduce((s, g) => s + g.count, 0);

  // When a card is hovered, cards below it slide down to reveal the full card
  const spreadAmount = cardHeight - peek;
  const getTop = (i: number) => {
    const base = i * peek;
    if (hoveredIdx === null || i <= hoveredIdx) return base;
    return base + spreadAmount;
  };

  const totalHeight = groups.length > 0
    ? (hoveredIdx !== null
      ? getTop(groups.length - 1) + cardHeight
      : peek * (groups.length - 1) + cardHeight)
    : 0;

  return (
    <div className="shrink-0 flex flex-col" style={{ width: cardWidth }}>
      <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2 truncate">
        {label} <span className="font-normal opacity-60">({count})</span>
      </div>
      <div className="relative transition-[height] duration-200 ease-out" style={{ height: totalHeight }}>
        {groups.map((g, i) => (
          <DraggableStackCard
            key={g.card.name}
            group={g}
            dragId={`deck-${sectionId}-${g.card.name}`}
            cardWidth={cardWidth}
            cardHeight={cardHeight}
            index={i}
            onAddOne={() => onAddOne(g)}
            onRemoveOne={() => onRemoveOne(g.card.name)}
            onHover={onHover}
            onLeave={onLeave}
            onUntag={onUntag ? () => onUntag(g.card.name) : undefined}
            isSelected={selectedCards?.has(g.card.name.toLowerCase())}
            onSelect={onSelectCard}
            onShowInfo={onShowInfo ? () => onShowInfo(g.card.name) : undefined}
            topOffset={getTop(i)}
            onCardHover={setHoveredIdx}
            onCardLeave={() => setHoveredIdx(null)}
          />
        ))}
      </div>
    </div>
  );
}

// ─── Visual Grid Card ─────────────────────────────────────────────────────────

interface CardVisualProps {
  group: CardGroup;
  dragId: string;
  onAddOne: () => void;
  onRemoveOne: () => void;
  onPickPrint: () => void;
  onHover: (x: number, y: number) => void;
  onLeave: () => void;
  isSelected?: boolean;
  onSelect?: (cardName: string, addToSelection: boolean) => void;
  onShowInfo?: () => void;
  onUntag?: () => void;
}

function CardVisual({
  group, dragId,
  onAddOne, onRemoveOne, onHover, onLeave,
  isSelected, onSelect, onShowInfo, onUntag,
}: CardVisualProps) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: dragId,
    data: { type: "deck-card", card: group.card, name: group.card.name },
  });

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={cn(
        "relative group cursor-grab active:cursor-grabbing select-none transition-[box-shadow]",
        isDragging && "opacity-30",
        isSelected && cn(CARD_RING.selected, "rounded-lg"),
      )}
      data-card-name={group.card.name}
      onMouseEnter={(e) => onHover(e.clientX, e.clientY)}
      onMouseMove={(e) => onHover(e.clientX, e.clientY)}
      onMouseLeave={onLeave}
      onClick={(e) => handleCardClick(e, group.card.name, onSelect, onShowInfo)}
    >
      <CardThumbnail imageUrl={group.card.imageUrl} name={group.card.name} />
      <CardCountBadge count={group.count} />
      <CardHoverOverlay actions={buildCardActions(onAddOne, onRemoveOne, onUntag)} />
    </div>
  );
}

// ─── List Row ─────────────────────────────────────────────────────────────────

interface CardRowProps {
  group: CardGroup;
  dragId: string;
  isCommander: boolean;
  onAddOne: () => void;
  onRemoveOne: () => void;
  onRemoveAll: () => void;
  onSetCommander: () => void;
  onRemoveCommander: () => void;
  onMoveToSide: () => void;
  onPickPrint: () => void;
  onHover: (x: number, y: number) => void;
  onLeave: () => void;
  isSelected?: boolean;
  onSelect?: (cardName: string, addToSelection: boolean) => void;
  onShowInfo?: () => void;
}

function CardRow({
  group, dragId, isCommander,
  onAddOne, onRemoveOne, onRemoveAll, onSetCommander, onRemoveCommander, onMoveToSide,
  onHover, onLeave,
  isSelected, onSelect, onShowInfo,
}: CardRowProps) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: dragId,
    data: { type: "deck-card", card: group.card, name: group.card.name },
  });

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={cn(
        "flex items-center gap-1 group hover:bg-muted/40 rounded px-1 py-0.5 cursor-grab active:cursor-grabbing select-none transition-colors",
        isDragging && "opacity-30",
        isSelected && "bg-selection/20"
      )}
      data-card-name={group.card.name}
      onMouseEnter={(e) => onHover(e.clientX, e.clientY)}
      onMouseMove={(e) => onHover(e.clientX, e.clientY)}
      onMouseLeave={onLeave}
      onClick={(e) => {
        e.stopPropagation();
        if (e.shiftKey && onSelect) {
          onSelect(group.card.name, true);
        } else if (onShowInfo) {
          onShowInfo();
        }
      }}
    >
      <div className={cn("w-4 h-4 rounded border flex items-center justify-center shrink-0 transition-colors", isSelected ? "bg-selection border-selection" : "border-muted-foreground/40")}>
        {isSelected && <span className="text-[8px] text-white font-bold">✓</span>}
      </div>
      <span className="text-xs font-mono w-4 text-right text-muted-foreground shrink-0">{group.count}</span>
      <span className="text-sm flex-1 truncate" title={group.card.name}>{group.card.name}</span>
      {group.card.manaCost && (
        <ManaSymbols cost={group.card.manaCost} size="sm" className="shrink-0" />
      )}
      {group.card.power && group.card.toughness && (
        <span className="text-xs text-muted-foreground/60 shrink-0 font-mono">
          {group.card.power}/{group.card.toughness}
        </span>
      )}
      <div className="flex gap-0.5 shrink-0 pointer-events-none group-hover:pointer-events-auto">
        <Button
          size="icon" variant="ghost"
          className={isCommander ? "h-5 w-5 text-commander" : "h-5 w-5 text-muted-foreground/40 opacity-0 group-hover:opacity-100 transition-opacity"}
          title={isCommander ? "Remove commander" : "Set as commander"}
          onClick={(e) => { e.stopPropagation(); isCommander ? onRemoveCommander() : onSetCommander(); }}
        >
          <Crown className="h-3 w-3" />
        </Button>
        <div className="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
          <Button size="icon" variant="ghost" className="h-5 w-5" title="Add one" onClick={(e) => { e.stopPropagation(); onAddOne(); }}>
            <Plus className="h-3 w-3" />
          </Button>
          <Button size="icon" variant="ghost" className="h-5 w-5" title="Remove one" onClick={(e) => { e.stopPropagation(); onRemoveOne(); }}>
            <Minus className="h-3 w-3" />
          </Button>
          <Button size="icon" variant="ghost" className="h-5 w-5 text-muted-foreground" title="Move to sideboard" onClick={(e) => { e.stopPropagation(); onMoveToSide(); }}>
            <Download className="h-3 w-3" />
          </Button>
          <Button size="icon" variant="ghost" className="h-5 w-5 text-destructive" title="Remove all" onClick={(e) => { e.stopPropagation(); onRemoveAll(); }}>
            <X className="h-3 w-3" />
          </Button>
        </div>
      </div>
    </div>
  );
}

// ─── Unified Collapsible Card Section ─────────────────────────────────────────

interface CardSectionProps {
  label: string;
  sectionId: string;
  groups: CardGroup[];
  commanderName?: string;
  viewMode: ViewMode;
  gridCols: string;
  onAddOne: (g: CardGroup) => void;
  onRemoveOne: (name: string) => void;
  onRemoveAll: (name: string) => void;
  onSetCommander: (card: Card) => void;
  onRemoveCommander: () => void;
  onMoveToSide: (name: string) => void;
  onPickPrint: (name: string) => void;
  onHover: (card: Card, x: number, y: number) => void;
  onLeave: () => void;
  selectedCards?: Set<string>;
  onSelectCard?: (cardName: string, addToSelection: boolean) => void;
  onShowInfo?: (cardName: string) => void;
  // Tag-specific props (optional)
  tag?: string;
  onUntagCard?: (cardName: string) => void;
  onRemoveTag?: () => void;
}

function CardSection({
  label, sectionId, groups, commanderName, viewMode, gridCols,
  onAddOne, onRemoveOne, onRemoveAll, onSetCommander, onRemoveCommander,
  onMoveToSide, onPickPrint, onHover, onLeave,
  selectedCards, onSelectCard, onShowInfo,
  tag, onUntagCard, onRemoveTag,
}: CardSectionProps) {
  const [collapsed, setCollapsed] = useState(false);
  const isTagSection = !!tag;
  const { setNodeRef, isOver } = useDroppable({
    id: isTagSection ? `${DROP_ZONE.TAG_PREFIX}${tag}` : `section-${sectionId}`,
    disabled: !isTagSection,
  });

  if (!isTagSection && groups.length === 0) return null;
  const count = groups.reduce((s, g) => s + g.count, 0);
  const dragPrefix = isTagSection ? `deck-tag-${tag}` : `deck-${sectionId}`;
  const effectiveRemoveOne = isTagSection && onUntagCard ? onUntagCard : onRemoveOne;

  const headerExtra = isTagSection && onRemoveTag ? (
    <Button
      size="icon" variant="ghost"
      className="h-5 w-5 text-destructive opacity-0 hover:opacity-100 transition-opacity shrink-0"
      title={`Remove "${tag}" tag`}
      onClick={onRemoveTag}
    >
      <X className="h-3 w-3" />
    </Button>
  ) : undefined;

  return (
    <div
      ref={isTagSection ? setNodeRef : undefined}
      className={cn("mb-3", isTagSection && "rounded-lg transition-colors p-1 -mx-1", isOver && "bg-primary/10")}
    >
      <CollapsibleHeader
        label={label}
        count={count}
        collapsed={collapsed}
        onToggle={() => setCollapsed((v) => !v)}
        extraContent={headerExtra}
      />

      {!collapsed && (
        groups.length === 0 ? (
          <EmptyDropZone message="Drag cards here" />
        ) : viewMode === "list" ? (
          <div className="space-y-0.5">
            {groups.map((g) => (
              <div key={g.card.name} className={cn("flex items-center gap-1", isTagSection && "group/tag")}>
                <div className="flex-1 min-w-0">
                  <CardRow
                    group={g}
                    dragId={`${dragPrefix}-${g.card.name}`}
                    isCommander={commanderName === g.card.name}
                    onAddOne={() => onAddOne(g)}
                    onRemoveOne={() => effectiveRemoveOne(g.card.name)}
                    onRemoveAll={() => onRemoveAll(g.card.name)}
                    onSetCommander={() => onSetCommander(g.card)}
                    onRemoveCommander={onRemoveCommander}
                    onMoveToSide={() => onMoveToSide(g.card.name)}
                    onPickPrint={() => onPickPrint(g.card.name)}
                    onHover={(x, y) => onHover(g.card, x, y)}
                    onLeave={onLeave}
                    isSelected={selectedCards?.has(g.card.name.toLowerCase())}
                    onSelect={onSelectCard}
                    onShowInfo={onShowInfo ? () => onShowInfo(g.card.name) : undefined}
                  />
                </div>
                {isTagSection && onUntagCard && (
                  <Button
                    size="icon" variant="ghost"
                    className="h-5 w-5 text-muted-foreground/40 opacity-0 group-hover/tag:opacity-100 transition-opacity shrink-0"
                    title="Remove from this tag"
                    onClick={() => onUntagCard(g.card.name)}
                  >
                    <Tag className="h-3 w-3" />
                  </Button>
                )}
              </div>
            ))}
          </div>
        ) : (
          <div className={cn("grid gap-2", gridCols)}>
            {groups.map((g) => (
              <CardVisual
                key={g.card.name}
                group={g}
                dragId={`${dragPrefix}-${g.card.name}`}
                onAddOne={() => onAddOne(g)}
                onRemoveOne={() => effectiveRemoveOne(g.card.name)}
                onUntag={isTagSection && onUntagCard ? () => onUntagCard(g.card.name) : undefined}
                onPickPrint={() => onPickPrint(g.card.name)}
                onHover={(x, y) => onHover(g.card, x, y)}
                onLeave={onLeave}
                isSelected={selectedCards?.has(g.card.name.toLowerCase())}
                onSelect={onSelectCard}
                onShowInfo={onShowInfo ? () => onShowInfo(g.card.name) : undefined}
              />
            ))}
          </div>
        )
      )}
    </div>
  );
}

// ─── Droppable Stack Tag Column ──────────────────────────────────────────────

function DroppableStackTag({
  tag, groups, cardWidth,
  onAddOne, onRemoveOne, onHover, onLeave, onRemoveTag, onUntagCard,
  selectedCards, onSelectCard,
}: {
  tag: string;
  groups: CardGroup[];
  cardWidth: number;
  onAddOne: (g: CardGroup) => void;
  onRemoveOne: (name: string) => void;
  onHover: (card: Card, x: number, y: number) => void;
  onLeave: () => void;
  onRemoveTag: () => void;
  onUntagCard?: (cardName: string, tag: string) => void;
  selectedCards?: Set<string>;
  onSelectCard?: (cardName: string, addToSelection: boolean) => void;
}) {
  const { setNodeRef, isOver } = useDroppable({ id: `${DROP_ZONE.TAG_PREFIX}${tag}` });

  return (
    <div
      ref={setNodeRef}
      className={cn("shrink-0 rounded-lg transition-colors p-2 -m-1 min-h-[160px]", isOver && "bg-primary/10 border-2 border-dashed border-primary/40")}
      style={{ minWidth: cardWidth + 8 }}
    >
      {groups.length > 0 ? (
        <StackColumn
          label={tag}
          sectionId={`tag-${tag}`}
          groups={groups}
          cardWidth={cardWidth}
          onAddOne={onAddOne}
          onRemoveOne={onRemoveOne}
          onHover={onHover}
          onLeave={onLeave}
          onUntag={onUntagCard ? (cardName) => onUntagCard(cardName, tag) : undefined}
          selectedCards={selectedCards}
          onSelectCard={onSelectCard}
        />
      ) : (
        <div className="flex flex-col h-full" style={{ width: cardWidth }}>
          <div className="flex items-center gap-1 mb-2">
            <Tag className="h-3 w-3 text-primary/60 shrink-0" />
            <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide truncate">{tag}</span>
            <Button
              size="icon"
              variant="ghost"
              className="h-4 w-4 text-destructive shrink-0 ml-auto"
              onClick={onRemoveTag}
            >
              <X className="h-2.5 w-2.5" />
            </Button>
          </div>
          <div className="border-2 border-dashed border-border/40 rounded-lg flex-1 flex items-center justify-center">
            <p className="text-[10px] text-muted-foreground/40 text-center">Drop cards here</p>
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Main DeckListView Component ──────────────────────────────────────────────

export interface DeckListViewProps {
  viewMode: ViewMode;
  cardSize: number;
  commander: Card | null;
  mainSections: Array<SectionDefinition & { groups: CardGroup[] }>;
  otherGroups: CardGroup[];
  sideGroups: CardGroup[];
  stackColumns: Array<SectionDefinition & { groups: CardGroup[] }>;
  isOverSide: boolean;
  setSideDropRef: (node: HTMLElement | null) => void;
  onAddOne: (g: CardGroup) => void;
  onRemoveOne: (name: string) => void;
  onRemoveAll: (name: string) => void;
  onSetCommander: (card: Card) => void;
  onRemoveCommander: () => void;
  onMoveToSide: (name: string) => void;
  onMoveToMain: (name: string) => void;
  onPickPrint: (name: string) => void;
  onHover: (card: Card, x: number, y: number) => void;
  onLeave: () => void;
  onAddToSide: (card: Card) => void;
  onRemoveFromSide: (name: string) => void;
  totalCards: number;
  customTags?: string[];
  cardTags?: Record<string, string[]>;
  allMainCards?: Card[];
  onUntagCard?: (cardName: string, tag: string) => void;
  onRemoveTag?: (tag: string) => void;
  selectedCards?: Set<string>;
  onSelectCard?: (cardName: string, addToSelection: boolean) => void;
  onSelectAll?: (cardNames: string[]) => void;
  onShowInfo?: (cardName: string) => void;
}

export function DeckListView({
  viewMode, cardSize, commander,
  mainSections, otherGroups, sideGroups, stackColumns,
  isOverSide, setSideDropRef,
  onAddOne, onRemoveOne, onRemoveAll, onSetCommander, onRemoveCommander,
  onMoveToSide, onMoveToMain, onPickPrint, onHover, onLeave,
  onAddToSide, onRemoveFromSide, totalCards,
  customTags, cardTags, allMainCards,
  onUntagCard, onRemoveTag,
  selectedCards, onSelectCard, onSelectAll,
  onShowInfo,
}: DeckListViewProps) {
  const [selectMode, setSelectMode] = useState(false);
  const gridCols = GRID_COLS[cardSize] ?? "grid-cols-8";
  const cardWidth = CARD_WIDTH_MAP[cardSize] ?? 115;

  const handleMarqueeComplete = useCallback((rect: { left: number; top: number; width: number; height: number }, additive: boolean) => {
    if (!containerRef.current || !onSelectAll) return;
    // Convert container-local marquee rect to viewport coordinates
    const containerRect = containerRef.current.getBoundingClientRect();
    const mLeft = rect.left + containerRect.left;
    const mTop = rect.top + containerRect.top;
    const mRight = mLeft + rect.width;
    const mBottom = mTop + rect.height;

    const cardEls = containerRef.current.querySelectorAll("[data-card-name]");
    const selected: string[] = [];
    cardEls.forEach((el) => {
      const elRect = el.getBoundingClientRect();
      if (elRect.right >= mLeft && elRect.left <= mRight && elRect.bottom >= mTop && elRect.top <= mBottom) {
        const name = el.getAttribute("data-card-name");
        if (name) selected.push(name);
      }
    });
    if (selected.length > 0) {
      onSelectAll(additive ? [...(selectedCards ?? []), ...selected] : selected);
    }
  }, [onSelectAll, selectedCards]);

  const { containerRef, marqueeRect, handleContainerMouseDown } = useMarquee({
    onMarqueeComplete: handleMarqueeComplete,
  });

  const wrappedHandleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement;
    if (target.closest("[data-card-name]")) return;
    if (!selectMode) {
      // In move mode, clicking background clears selection
      if (onSelectAll) onSelectAll([]);
      return;
    }
    handleContainerMouseDown(e);
  }, [selectMode, handleContainerMouseDown, onSelectAll]);

  const sharedSectionProps = {
    commanderName: commander?.name,
    viewMode,
    gridCols,
    onAddOne,
    onRemoveOne,
    onRemoveAll,
    onSetCommander,
    onRemoveCommander,
    onMoveToSide,
    onPickPrint,
    onHover,
    onLeave,
    selectedCards,
    onSelectCard,
    onShowInfo,
  };

  const selectModeControls = (
    <div className="absolute top-1 right-1 z-40 flex items-center gap-1">
      {(selectedCards?.size ?? 0) > 0 && (
        <span className="text-[10px] px-1.5 py-0.5 rounded bg-card/90 border text-selection">
          {selectedCards!.size} selected
        </span>
      )}
      <div className="flex gap-0.5 rounded bg-card/90 border p-0.5 shadow-sm">
        <button
          title="Move mode"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={() => setSelectMode(false)}
          className={cn(
            "p-0.5 rounded transition-colors",
            !selectMode
              ? "text-foreground bg-muted"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          <Move size={12} />
        </button>
        <button
          title="Select mode — drag to rubber-band select cards"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={() => setSelectMode(true)}
          className={cn(
            "p-0.5 rounded transition-colors",
            selectMode
              ? "text-foreground bg-muted"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          <MousePointer2 size={12} />
        </button>
      </div>
    </div>
  );

  const marqueeOverlay = marqueeRect && (
    <div
      className="absolute pointer-events-none border-2 border-dashed border-selection bg-selection/10 z-[9999] rounded"
      style={{
        left: marqueeRect.left,
        top: marqueeRect.top,
        width: marqueeRect.width,
        height: marqueeRect.height,
      }}
    />
  );

  if (viewMode === "stack") {
    return (
      <div
        ref={containerRef}
        className={cn("h-full overflow-auto relative", selectMode && "cursor-crosshair")}
        onMouseDown={wrappedHandleMouseDown}
      >
        {selectModeControls}
        <div className="flex gap-5 items-start p-3 min-w-max">
          {commander && (
            <StackColumn
              label="Commander"
              sectionId="commander"
              groups={[{ card: commander, count: 1 }]}
              cardWidth={cardWidth}
              onAddOne={() => {}}
              onRemoveOne={onRemoveCommander}
              onHover={onHover}
              onLeave={onLeave}
            />
          )}

          {stackColumns.map((col) => (
            <StackColumn
              key={col.id}
              label={col.label}
              sectionId={col.id}
              groups={col.groups}
              cardWidth={cardWidth}
              onAddOne={onAddOne}
              onRemoveOne={onRemoveOne}
              onHover={onHover}
              onLeave={onLeave}
              selectedCards={selectedCards}
              onSelectCard={onSelectCard}
              onShowInfo={onShowInfo}
            />
          ))}

          {customTags && customTags.length > 0 && allMainCards && customTags.map((tag) => {
            const tagGroups = getTaggedGroups(tag, allMainCards, cardTags);
            return (
              <DroppableStackTag
                key={tag}
                tag={tag}
                groups={tagGroups}
                cardWidth={cardWidth}
                onAddOne={onAddOne}
                onRemoveOne={onRemoveOne}
                onHover={onHover}
                onLeave={onLeave}
                onRemoveTag={() => onRemoveTag?.(tag)}
                onUntagCard={onUntagCard ?? undefined}
                selectedCards={selectedCards}
                onSelectCard={onSelectCard}
              />
            );
          })}

          <div
            ref={setSideDropRef}
            className={cn("shrink-0 rounded-lg transition-colors p-2 -m-1 min-h-[160px]", isOverSide && "bg-primary/10 border-2 border-dashed border-primary/40")}
            style={{ minWidth: cardWidth + 8 }}
          >
            {sideGroups.length > 0 ? (
              <StackColumn
                label="Sideboard"
                sectionId="sideboard"
                groups={sideGroups}
                cardWidth={cardWidth}
                onAddOne={(g) => onAddToSide({ ...g.card, id: crypto.randomUUID() })}
                onRemoveOne={onRemoveFromSide}
                onHover={onHover}
                onLeave={onLeave}
              />
            ) : (
              <div className="flex flex-col h-full" style={{ width: cardWidth }}>
                <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Sideboard</div>
                <div className="border-2 border-dashed border-border/40 rounded-lg flex-1 flex items-center justify-center">
                  <p className="text-[10px] text-muted-foreground/40 text-center">Drop cards here</p>
                </div>
              </div>
            )}
          </div>
        </div>
        {marqueeOverlay}
      </div>
    );
  }

  return (
    <div className="h-full relative">
      {selectModeControls}
      {marqueeRect && (
        <div
          className="absolute pointer-events-none border-2 border-dashed border-selection bg-selection/10 z-[9999] rounded"
          style={{
            left: marqueeRect.left,
            top: marqueeRect.top,
            width: marqueeRect.width,
            height: marqueeRect.height,
          }}
        />
      )}
    <ScrollArea
      ref={containerRef}
      className={cn("h-full px-3 py-2 relative", selectMode && "cursor-crosshair")}
      onMouseDown={wrappedHandleMouseDown}
    >
      {commander && (() => {
        const cmdGroup: CardGroup = { card: commander, count: 1 };
        return (
          <div className="mb-3">
            <div className="flex items-center gap-1 mb-1.5">
              <Crown className="h-3 w-3 text-commander shrink-0" />
              <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">Commander</span>
            </div>
            {viewMode === "list" ? (
              <div
                className="flex items-center gap-1 group hover:bg-muted/40 rounded px-1 py-0.5"
                onMouseEnter={(e) => onHover(commander, e.clientX, e.clientY)}
                onMouseMove={(e) => onHover(commander, e.clientX, e.clientY)}
                onMouseLeave={onLeave}
              >
                <Crown className="h-3 w-3 text-commander shrink-0" />
                <span className="text-sm flex-1 truncate">{commander.name}</span>
                {commander.manaCost && (
                  <ManaSymbols cost={commander.manaCost} size="sm" className="shrink-0" />
                )}
                <Button size="icon" variant="ghost" className="h-5 w-5 text-destructive opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                  onClick={onRemoveCommander}>
                  <X className="h-3 w-3" />
                </Button>
              </div>
            ) : (
              <div className={cn("grid gap-2", gridCols)}>
                <div className="relative">
                  <div className="absolute top-1 right-1 z-20 bg-overlay/70 rounded-full p-0.5 shadow">
                    <Crown className="h-3.5 w-3.5 text-commander" />
                  </div>
                  <CardVisual
                    group={cmdGroup}
                    dragId="deck-commander"
                    onAddOne={() => {}}
                    onRemoveOne={onRemoveCommander}
                    onPickPrint={() => onPickPrint(commander.name)}
                    onHover={(x, y) => onHover(commander, x, y)}
                    onLeave={onLeave}
                  />
                </div>
              </div>
            )}
          </div>
        );
      })()}

      {totalCards === 0 && (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <div className="text-4xl mb-3 opacity-20">🃏</div>
          <p className="text-sm text-muted-foreground">Drag cards here from the search panel</p>
          <p className="text-xs text-muted-foreground/60 mt-1">or use the + buttons on hover</p>
        </div>
      )}

      {viewMode === "list" ? (
        <>
          {/* Multi-column layout for list view */}
          <div className="columns-3 gap-4">
            {mainSections.map((s) => (
              <div key={s.id} className="break-inside-avoid">
                <CardSection
                  label={s.label}
                  groups={s.groups}
                  sectionId={s.id}
                  {...sharedSectionProps}
                />
              </div>
            ))}
            {otherGroups.length > 0 && (
              <div className="break-inside-avoid">
                <CardSection
                  label="Other"
                  groups={otherGroups}
                  sectionId="other"
                  {...sharedSectionProps}
                />
              </div>
            )}
          </div>

          {customTags && customTags.length > 0 && allMainCards && (
            <>
              <div className="border-t border-border/30 my-3" />
              <div className="columns-3 gap-4">
                {customTags.map((tag) => {
                  const tagGroups = getTaggedGroups(tag, allMainCards, cardTags);
                  return (
                    <div key={tag} className="break-inside-avoid">
                      <CardSection
                        label={tag}
                        sectionId={`tag-${tag}`}
                        tag={tag}
                        groups={tagGroups}
                        {...sharedSectionProps}
                        onUntagCard={(cardName) => onUntagCard?.(cardName, tag)}
                        onRemoveTag={() => onRemoveTag?.(tag)}
                      />
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </>
      ) : (
        <>
          {mainSections.map((s) => (
            <CardSection
              key={s.id}
              label={s.label}
              groups={s.groups}
              sectionId={s.id}
              {...sharedSectionProps}
            />
          ))}

          {otherGroups.length > 0 && (
            <CardSection
              label="Other"
              groups={otherGroups}
              sectionId="other"
              {...sharedSectionProps}
            />
          )}

          {customTags && customTags.length > 0 && allMainCards && (
            <>
              <div className="border-t border-border/30 my-3" />
              {customTags.map((tag) => {
                const tagGroups = getTaggedGroups(tag, allMainCards, cardTags);
                return (
                  <CardSection
                    key={tag}
                    label={tag}
                    sectionId={`tag-${tag}`}
                    tag={tag}
                    groups={tagGroups}
                    {...sharedSectionProps}
                    onUntagCard={(cardName) => onUntagCard?.(cardName, tag)}
                    onRemoveTag={() => onRemoveTag?.(tag)}
                  />
                );
              })}
            </>
          )}
        </>
      )}

      <div
        ref={setSideDropRef}
        className={cn(
          "mt-2 rounded-lg border-2 border-dashed transition-colors",
          isOverSide ? "border-primary bg-primary/10" : "border-border/40 hover:border-border/60"
        )}
      >
        <div className="px-2 pt-2 pb-1">
          <div className="flex items-center gap-2 mb-1.5">
            <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
              Sideboard ({sideGroups.reduce((s, g) => s + g.count, 0)})
            </span>
            <span className="text-xs text-muted-foreground/40">— drop cards here</span>
          </div>
          {sideGroups.length === 0 ? (
            <div className="py-4 text-center">
              <p className="text-xs text-muted-foreground/40">Drag cards here for sideboard</p>
            </div>
          ) : viewMode === "list" ? (
            <div className="space-y-0.5 pb-1">
              {sideGroups.map((g) => (
                <div
                  key={g.card.name}
                  className="flex items-center gap-1 group hover:bg-muted/40 rounded px-1 py-0.5"
                  onMouseEnter={(e) => onHover(g.card, e.clientX, e.clientY)}
                  onMouseMove={(e) => onHover(g.card, e.clientX, e.clientY)}
                  onMouseLeave={onLeave}
                >
                  <span className="text-xs font-mono w-4 text-right text-muted-foreground shrink-0">{g.count}</span>
                  <span className="text-sm flex-1 truncate">{g.card.name}</span>
                  {g.card.manaCost && <ManaSymbols cost={g.card.manaCost} size="sm" className="shrink-0" />}
                  <div className="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                    <Button size="icon" variant="ghost" className="h-5 w-5 text-muted-foreground" title="Move to main" onClick={() => onMoveToMain(g.card.name)}>
                      <Upload className="h-3 w-3" />
                    </Button>
                    <Button size="icon" variant="ghost" className="h-5 w-5 text-destructive" onClick={() => onRemoveFromSide(g.card.name)}>
                      <X className="h-3 w-3" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className={cn("grid gap-2 pb-1", gridCols)}>
              {sideGroups.map((g) => (
                <div
                  key={g.card.name}
                  className="relative group"
                  onMouseEnter={(e) => onHover(g.card, e.clientX, e.clientY)}
                  onMouseMove={(e) => onHover(g.card, e.clientX, e.clientY)}
                  onMouseLeave={onLeave}
                >
                  {g.card.imageUrl ? (
                    <img src={g.card.imageUrl} alt={g.card.name} className="w-full rounded-lg border border-border/50" draggable={false} />
                  ) : (
                    <div className="w-full aspect-[2.5/3.5] rounded-lg border border-border bg-muted flex items-center justify-center p-2">
                      <span className="text-xs text-center text-muted-foreground">{g.card.name}</span>
                    </div>
                  )}
                  {g.count > 1 && (
                    <div className="absolute top-1 left-1 bg-overlay/80 text-white text-[10px] font-bold rounded-full w-5 h-5 flex items-center justify-center">
                      {g.count}
                    </div>
                  )}
                  <div className="absolute inset-0 bg-overlay/60 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex flex-col items-center justify-center gap-1 pointer-events-none group-hover:pointer-events-auto">
                    <Button size="sm" variant="secondary" className="h-6 w-4/5 text-xs" onClick={() => onMoveToMain(g.card.name)}>
                      → Main
                    </Button>
                    <Button size="sm" variant="ghost" className="h-6 w-4/5 text-xs text-white/80 hover:text-white" onClick={() => onRemoveFromSide(g.card.name)}>
                      <X className="h-3 w-3 mr-1" /> Remove
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </ScrollArea>
    </div>
  );
}
