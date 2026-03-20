import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import {
  X, Minus, Plus, Download, Upload, Crown,
  ChevronDown, ChevronRight, Tag,
} from "lucide-react";
import { Image as ImageIcon } from "lucide-react";
import { useState } from "react";
import { useDraggable, useDroppable } from "@dnd-kit/core";
import { cn } from "@/lib/utils";
import type { Card } from "@/types/xmage";
import type { CardGroup, ViewMode, SectionDefinition } from "./deckBuilder.utils";
import { CARD_WIDTH_MAP, groupCards, getTaggedGroups } from "./deckBuilder.utils";
import { DROP_ZONE } from "@/lib/constants";

// ─── Draggable Stack Card ─────────────────────────────────────────────────────

function DraggableStackCard({
  group, dragId, cardWidth, cardHeight, peek, index,
  onAddOne, onRemoveOne, onHover, onLeave,
}: {
  group: CardGroup;
  dragId: string;
  cardWidth: number;
  cardHeight: number;
  peek: number;
  index: number;
  onAddOne: () => void;
  onRemoveOne: () => void;
  onHover: (card: Card, x: number, y: number) => void;
  onLeave: () => void;
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
      className={cn("absolute left-0 group cursor-grab active:cursor-grabbing", isDragging && "opacity-30")}
      style={{ top: index * peek, width: cardWidth, zIndex: index + 1 }}
      onMouseEnter={(e) => onHover(group.card, e.clientX, e.clientY)}
      onMouseMove={(e) => onHover(group.card, e.clientX, e.clientY)}
      onMouseLeave={onLeave}
    >
      {group.card.imageUrl ? (
        <img
          src={group.card.imageUrl}
          alt={group.card.name}
          className="w-full block rounded-[4%] shadow-sm border border-border/40"
          draggable={false}
        />
      ) : (
        <div
          className="bg-muted border border-border rounded-[4%] p-1 flex flex-col"
          style={{ width: cardWidth, height: cardHeight }}
        >
          <span className="text-[9px] text-muted-foreground leading-tight">{group.card.name}</span>
        </div>
      )}
      {group.count > 1 && (
        <div className="absolute top-1 left-1 bg-black/80 text-white text-[10px] font-bold rounded-full w-5 h-5 flex items-center justify-center border border-white/30 shadow" style={{ zIndex: 10 }}>
          {group.count}
        </div>
      )}
      <div className="absolute inset-0 rounded-[4%] bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col items-center justify-center gap-1 pointer-events-none group-hover:pointer-events-auto">
        <Button size="sm" variant="secondary" className="h-6 w-4/5 text-xs" onClick={(e) => { e.stopPropagation(); onAddOne(); }}>
          <Plus className="h-3 w-3 mr-1" /> Add
        </Button>
        <Button size="sm" variant="ghost" className="h-6 w-4/5 text-xs text-white/80 hover:text-white hover:bg-white/10" onClick={(e) => { e.stopPropagation(); onRemoveOne(); }}>
          <Minus className="h-3 w-3 mr-1" /> Remove
        </Button>
      </div>
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
}

function StackColumn({
  label, sectionId, groups, cardWidth,
  onAddOne, onRemoveOne, onHover, onLeave,
}: StackColumnProps) {
  const cardHeight = Math.round(cardWidth * 1.4);
  const peek = Math.round(cardHeight * 0.28);
  const totalHeight = groups.length > 0 ? peek * (groups.length - 1) + cardHeight : 0;
  const count = groups.reduce((s, g) => s + g.count, 0);

  return (
    <div className="shrink-0 flex flex-col" style={{ width: cardWidth }}>
      <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2 truncate">
        {label} <span className="font-normal opacity-60">({count})</span>
      </div>
      <div className="relative" style={{ height: totalHeight }}>
        {groups.map((g, i) => (
          <DraggableStackCard
            key={g.card.name}
            group={g}
            dragId={`deck-${sectionId}-${g.card.name}`}
            cardWidth={cardWidth}
            cardHeight={cardHeight}
            peek={peek}
            index={i}
            onAddOne={() => onAddOne(g)}
            onRemoveOne={() => onRemoveOne(g.card.name)}
            onHover={onHover}
            onLeave={onLeave}
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
}

function CardVisual({
  group, dragId,
  onAddOne, onRemoveOne, onPickPrint, onHover, onLeave,
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
      className={cn("relative group cursor-grab active:cursor-grabbing select-none", isDragging && "opacity-30")}
      onMouseEnter={(e) => onHover(e.clientX, e.clientY)}
      onMouseMove={(e) => onHover(e.clientX, e.clientY)}
      onMouseLeave={onLeave}
    >
      {group.card.imageUrl ? (
        <img
          src={group.card.imageUrl}
          alt={group.card.name}
          className="w-full rounded-lg border border-border/50 shadow-sm"
          draggable={false}
        />
      ) : (
        <div className="w-full aspect-[2.5/3.5] rounded-lg border border-border bg-muted flex items-center justify-center p-2">
          <span className="text-xs text-center text-muted-foreground font-medium leading-tight">{group.card.name}</span>
        </div>
      )}
      {group.count > 1 && (
        <div className="absolute top-1 left-1 bg-black/80 text-white text-[10px] font-bold rounded-full w-5 h-5 flex items-center justify-center border border-white/20">
          {group.count}
        </div>
      )}
      <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex flex-col items-center justify-center gap-1 pointer-events-none group-hover:pointer-events-auto">
        <Button size="sm" variant="secondary" className="h-6 w-4/5 text-xs" onClick={(e) => { e.stopPropagation(); onAddOne(); }}>
          <Plus className="h-3 w-3 mr-1" /> Add
        </Button>
        <Button size="sm" variant="ghost" className="h-6 w-4/5 text-xs text-white/80 hover:text-white hover:bg-white/10" onClick={(e) => { e.stopPropagation(); onRemoveOne(); }}>
          <Minus className="h-3 w-3 mr-1" /> Remove
        </Button>
        <Button size="sm" variant="ghost" className="h-6 w-4/5 text-xs text-white/80 hover:text-white hover:bg-white/10" onClick={(e) => { e.stopPropagation(); onPickPrint(); }}>
          <ImageIcon className="h-3 w-3 mr-1" /> Print
        </Button>
      </div>
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
}

function CardRow({
  group, dragId, isCommander,
  onAddOne, onRemoveOne, onRemoveAll, onSetCommander, onRemoveCommander, onMoveToSide, onPickPrint,
  onHover, onLeave,
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
        "flex items-center gap-1 group hover:bg-muted/40 rounded px-1 py-0.5 cursor-grab active:cursor-grabbing select-none",
        isDragging && "opacity-30"
      )}
      onMouseEnter={(e) => onHover(e.clientX, e.clientY)}
      onMouseMove={(e) => onHover(e.clientX, e.clientY)}
      onMouseLeave={onLeave}
    >
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
          className={isCommander ? "h-5 w-5 text-yellow-500" : "h-5 w-5 text-muted-foreground/40 opacity-0 group-hover:opacity-100 transition-opacity"}
          title={isCommander ? "Remove commander" : "Set as commander"}
          onClick={(e) => { e.stopPropagation(); isCommander ? onRemoveCommander() : onSetCommander(); }}
        >
          <Crown className="h-3 w-3" />
        </Button>
        <div className="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
          <Button size="icon" variant="ghost" className="h-5 w-5" title="Add one" onClick={(e) => { e.stopPropagation(); onAddOne(); }}>
            <Plus className="h-3 w-3" />
          </Button>
          <Button size="icon" variant="ghost" className="h-5 w-5 text-muted-foreground" title="Change Print" onClick={(e) => { e.stopPropagation(); onPickPrint(); }}>
            <ImageIcon className="h-3 w-3" />
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

// ─── Collapsible Section ──────────────────────────────────────────────────────

interface DeckSectionProps {
  label: string;
  groups: CardGroup[];
  commanderName?: string;
  viewMode: ViewMode;
  sectionId: string;
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
}

function DeckSection({
  label, groups, commanderName, viewMode, sectionId, gridCols,
  onAddOne, onRemoveOne, onRemoveAll, onSetCommander, onRemoveCommander, onMoveToSide, onPickPrint,
  onHover, onLeave,
}: DeckSectionProps) {
  const [collapsed, setCollapsed] = useState(false);
  if (groups.length === 0) return null;
  const count = groups.reduce((s, g) => s + g.count, 0);

  return (
    <div className="mb-3">
      <button
        type="button"
        className="flex items-center gap-1 w-full text-left mb-1.5 hover:text-foreground text-muted-foreground"
        onClick={() => setCollapsed((v) => !v)}
      >
        {collapsed ? <ChevronRight className="h-3 w-3 shrink-0" /> : <ChevronDown className="h-3 w-3 shrink-0" />}
        <span className="text-xs font-semibold uppercase tracking-wide">{label}</span>
        <span className="text-xs text-muted-foreground/60 ml-1">({count})</span>
      </button>

      {!collapsed && (
        viewMode === "list" ? (
          <div className="space-y-0.5">
            {groups.map((g) => (
              <CardRow
                key={g.card.name}
                group={g}
                dragId={`deck-${sectionId}-${g.card.name}`}
                isCommander={commanderName === g.card.name}
                onAddOne={() => onAddOne(g)}
                onRemoveOne={() => onRemoveOne(g.card.name)}
                onRemoveAll={() => onRemoveAll(g.card.name)}
                onSetCommander={() => onSetCommander(g.card)}
                onRemoveCommander={onRemoveCommander}
                onMoveToSide={() => onMoveToSide(g.card.name)}
                onPickPrint={() => onPickPrint(g.card.name)}
                onHover={(x, y) => onHover(g.card, x, y)}
                onLeave={onLeave}
              />
            ))}
          </div>
        ) : (
          <div className={cn("grid gap-2", gridCols)}>
            {groups.map((g) => (
              <CardVisual
                key={g.card.name}
                group={g}
                dragId={`deck-${sectionId}-${g.card.name}`}
                onAddOne={() => onAddOne(g)}
                onRemoveOne={() => onRemoveOne(g.card.name)}
                onPickPrint={() => onPickPrint(g.card.name)}
                onHover={(x, y) => onHover(g.card, x, y)}
                onLeave={onLeave}
              />
            ))}
          </div>
        )
      )}
    </div>
  );
}

// ─── Custom Tag Section (droppable) ──────────────────────────────────────────

interface TagSectionProps {
  tag: string;
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
  onUntagCard: (cardName: string) => void;
  onRemoveTag: () => void;
}

function TagSection({
  tag, groups, commanderName, viewMode, gridCols,
  onAddOne, onRemoveOne, onRemoveAll, onSetCommander, onRemoveCommander,
  onMoveToSide, onPickPrint, onHover, onLeave, onUntagCard, onRemoveTag,
}: TagSectionProps) {
  const [collapsed, setCollapsed] = useState(false);
  const { setNodeRef, isOver } = useDroppable({ id: `${DROP_ZONE.TAG_PREFIX}${tag}` });
  const count = groups.reduce((s, g) => s + g.count, 0);

  return (
    <div
      ref={setNodeRef}
      className={cn("mb-3 rounded-lg transition-colors p-1 -mx-1", isOver && "bg-primary/10")}
    >
      <div className="flex items-center gap-1 mb-1.5">
        <button
          type="button"
          className="flex items-center gap-1 flex-1 text-left hover:text-foreground text-muted-foreground"
          onClick={() => setCollapsed((v) => !v)}
        >
          {collapsed ? <ChevronRight className="h-3 w-3 shrink-0" /> : <ChevronDown className="h-3 w-3 shrink-0" />}
          <Tag className="h-3 w-3 shrink-0 text-primary/60" />
          <span className="text-xs font-semibold uppercase tracking-wide">{tag}</span>
          <span className="text-xs text-muted-foreground/60 ml-1">({count})</span>
        </button>
        <Button
          size="icon"
          variant="ghost"
          className="h-5 w-5 text-destructive opacity-0 hover:opacity-100 transition-opacity shrink-0"
          title={`Remove "${tag}" tag`}
          onClick={onRemoveTag}
        >
          <X className="h-3 w-3" />
        </Button>
      </div>

      {!collapsed && (
        groups.length === 0 ? (
          <div className="border border-dashed border-border/40 rounded py-3 text-center">
            <p className="text-[10px] text-muted-foreground/40">Drag cards here</p>
          </div>
        ) : viewMode === "list" ? (
          <div className="space-y-0.5">
            {groups.map((g) => (
              <div key={g.card.name} className="flex items-center gap-1">
                <div className="flex-1 min-w-0">
                  <CardRow
                    group={g}
                    dragId={`deck-tag-${tag}-${g.card.name}`}
                    isCommander={commanderName === g.card.name}
                    onAddOne={() => onAddOne(g)}
                    onRemoveOne={() => onRemoveOne(g.card.name)}
                    onRemoveAll={() => onRemoveAll(g.card.name)}
                    onSetCommander={() => onSetCommander(g.card)}
                    onRemoveCommander={onRemoveCommander}
                    onMoveToSide={() => onMoveToSide(g.card.name)}
                    onPickPrint={() => onPickPrint(g.card.name)}
                    onHover={(x, y) => onHover(g.card, x, y)}
                    onLeave={onLeave}
                  />
                </div>
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-5 w-5 text-muted-foreground/40 opacity-0 hover:opacity-100 transition-opacity shrink-0"
                  title="Remove from this tag"
                  onClick={() => onUntagCard(g.card.name)}
                >
                  <Tag className="h-3 w-3" />
                </Button>
              </div>
            ))}
          </div>
        ) : (
          <div className={cn("grid gap-2", gridCols)}>
            {groups.map((g) => (
              <CardVisual
                key={g.card.name}
                group={g}
                dragId={`deck-tag-${tag}-${g.card.name}`}
                onAddOne={() => onAddOne(g)}
                onRemoveOne={() => onRemoveOne(g.card.name)}
                onPickPrint={() => onPickPrint(g.card.name)}
                onHover={(x, y) => onHover(g.card, x, y)}
                onLeave={onLeave}
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
  onAddOne, onRemoveOne, onHover, onLeave, onRemoveTag,
}: {
  tag: string;
  groups: CardGroup[];
  cardWidth: number;
  onAddOne: (g: CardGroup) => void;
  onRemoveOne: (name: string) => void;
  onHover: (card: Card, x: number, y: number) => void;
  onLeave: () => void;
  onRemoveTag: () => void;
}) {
  const { setNodeRef, isOver } = useDroppable({ id: `${DROP_ZONE.TAG_PREFIX}${tag}` });

  return (
    <div
      ref={setNodeRef}
      className={cn("shrink-0 rounded-lg transition-colors p-1 -m-1", isOver && "bg-primary/10")}
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
        />
      ) : (
        <div className="flex flex-col" style={{ width: cardWidth }}>
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
          <div className="border-2 border-dashed border-border/40 rounded-lg py-6 flex items-center justify-center">
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
}: DeckListViewProps) {
  const gridCols = { 1: "grid-cols-5", 2: "grid-cols-4", 3: "grid-cols-3", 4: "grid-cols-2", 5: "grid-cols-1" }[cardSize] ?? "grid-cols-3";
  const cardWidth = CARD_WIDTH_MAP[cardSize] ?? 115;

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
  };

  if (viewMode === "stack") {
    return (
      <div className="h-full overflow-auto">
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
              />
            );
          })}

          <div
            ref={setSideDropRef}
            className={cn("shrink-0 rounded-lg transition-colors p-1 -m-1", isOverSide && "bg-primary/10")}
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
              <div className="flex flex-col" style={{ width: cardWidth }}>
                <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Sideboard</div>
                <div className="border-2 border-dashed border-border/40 rounded-lg py-6 flex items-center justify-center">
                  <p className="text-[10px] text-muted-foreground/40 text-center">Drop cards here</p>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  return (
    <ScrollArea className="h-full px-3 py-2">
      {commander && (() => {
        const cmdGroup: CardGroup = { card: commander, count: 1 };
        return (
          <div className="mb-3">
            <div className="flex items-center gap-1 mb-1.5">
              <Crown className="h-3 w-3 text-yellow-500 shrink-0" />
              <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">Commander</span>
            </div>
            {viewMode === "list" ? (
              <div
                className="flex items-center gap-1 group hover:bg-muted/40 rounded px-1 py-0.5"
                onMouseEnter={(e) => onHover(commander, e.clientX, e.clientY)}
                onMouseMove={(e) => onHover(commander, e.clientX, e.clientY)}
                onMouseLeave={onLeave}
              >
                <Crown className="h-3 w-3 text-yellow-500 shrink-0" />
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
                  <div className="absolute top-1 right-1 z-20 bg-black/70 rounded-full p-0.5 shadow">
                    <Crown className="h-3.5 w-3.5 text-yellow-400" />
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

      {mainSections.map((s) => (
        <DeckSection
          key={s.id}
          label={s.label}
          groups={s.groups}
          sectionId={s.id}
          {...sharedSectionProps}
        />
      ))}

      {otherGroups.length > 0 && (
        <DeckSection
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
              <TagSection
                key={tag}
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
                    <div className="absolute top-1 left-1 bg-black/80 text-white text-[10px] font-bold rounded-full w-5 h-5 flex items-center justify-center">
                      {g.count}
                    </div>
                  )}
                  <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex flex-col items-center justify-center gap-1 pointer-events-none group-hover:pointer-events-auto">
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
  );
}
