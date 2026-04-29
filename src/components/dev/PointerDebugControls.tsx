import { cn } from "@/lib/utils";
import { useGameDevStore } from "@/stores/useGameDevStore";
import type { ArrowType } from "@/pixi/types";
import { TargetingIntent, intentIsHostile } from "@/types/promptType";

const ARROW_TYPES: { type: ArrowType; label: string }[] = [
  { type: "attack", label: "attack" },
  { type: "block", label: "block" },
  { type: "attach", label: "attach" },
  { type: "placement", label: "placement" },
];

/** Label + grouping metadata for every pointer intent that PointerLayer
 *  has a glyph for. Combat intents (`attack` / `block`) live on
 *  ArrowLayer instead and are intentionally omitted. Order matches the
 *  manifest in `src/pixi/PointerLayer.ts`. */
const POINTER_INTENTS: { intent: TargetingIntent; label: string }[] = [
  { intent: TargetingIntent.Damage, label: "damage" },
  { intent: TargetingIntent.Destroy, label: "destroy" },
  { intent: TargetingIntent.Sacrifice, label: "sacrifice" },
  { intent: TargetingIntent.Exile, label: "exile" },
  { intent: TargetingIntent.Bounce, label: "bounce" },
  { intent: TargetingIntent.Mill, label: "mill" },
  { intent: TargetingIntent.Discard, label: "discard" },
  { intent: TargetingIntent.Counter, label: "counter" },
  { intent: TargetingIntent.Tap, label: "tap" },
  { intent: TargetingIntent.Debuff, label: "debuff" },
  { intent: TargetingIntent.LoseLife, label: "loseLife" },
  { intent: TargetingIntent.GainControl, label: "gainControl" },
  { intent: TargetingIntent.Fight, label: "fight" },
  { intent: TargetingIntent.Hostile, label: "hostile" },
  { intent: TargetingIntent.Untap, label: "untap" },
  { intent: TargetingIntent.Copy, label: "copy" },
  { intent: TargetingIntent.Buff, label: "buff" },
  { intent: TargetingIntent.Heal, label: "heal" },
  { intent: TargetingIntent.Reveal, label: "reveal" },
  { intent: TargetingIntent.Draw, label: "draw" },
  { intent: TargetingIntent.Friendly, label: "friendly" },
];

/** Dev panel section that force-renders one pointer (or one arrow) on
 *  the live game board so each glyph / arrow style can be inspected
 *  without setting up a real spell or combat. Both groups behave as
 *  radios — picking another intent replaces the previous one so visuals
 *  never overlap. Click the active selection to deselect. */
export function PointerDebugControls() {
  const selectedIntent = useGameDevStore((s) => s.debugPointerIntent);
  const setSelectedIntent = useGameDevStore((s) => s.setDebugPointerIntent);
  const selectedArrow = useGameDevStore((s) => s.debugArrowType);
  const setSelectedArrow = useGameDevStore((s) => s.setDebugArrowType);

  const onPickIntent = (intent: TargetingIntent) => {
    setSelectedIntent(selectedIntent === intent ? null : intent);
  };
  const onPickArrow = (type: ArrowType) => {
    setSelectedArrow(selectedArrow === type ? null : type);
  };
  const clearAll = () => {
    setSelectedIntent(null);
    setSelectedArrow(null);
  };

  const hostileIntents = POINTER_INTENTS.filter((p) => intentIsHostile(p.intent));
  const friendlyIntents = POINTER_INTENTS.filter((p) => !intentIsHostile(p.intent));
  const dirty = selectedIntent != null || selectedArrow != null;

  return (
    <div className="flex flex-col gap-2 mt-2 rounded-md border border-border/70 p-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Pointer / arrow debug
        </span>
        {dirty && (
          <button
            className="text-[10px] uppercase text-muted-foreground hover:text-destructive"
            onClick={clearAll}
          >
            Clear
          </button>
        )}
      </div>
      <p className="text-[10px] text-muted-foreground">
        Picks render a fake pointer/arrow (you → opponent) so you can inspect each style live. One
        of each — click the active one to clear.
      </p>
      <IntentGroup
        title="Hostile pointers"
        accent="text-rose-400"
        intents={hostileIntents}
        selected={selectedIntent}
        onPick={onPickIntent}
      />
      <IntentGroup
        title="Friendly pointers"
        accent="text-emerald-400"
        intents={friendlyIntents}
        selected={selectedIntent}
        onPick={onPickIntent}
      />
      <ArrowGroup selected={selectedArrow} onPick={onPickArrow} />
    </div>
  );
}

interface ArrowGroupProps {
  selected: ArrowType | null;
  onPick: (type: ArrowType) => void;
}

function ArrowGroup({ selected, onPick }: ArrowGroupProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-[10px] uppercase tracking-wide text-amber-400">Arrows</span>
      <div className="grid grid-cols-2 gap-1">
        {ARROW_TYPES.map(({ type, label }) => {
          const active = selected === type;
          return (
            <button
              key={type}
              className={cn(
                "px-2 py-1 rounded text-[11px] font-medium border text-left",
                active
                  ? "border-primary text-primary bg-primary/10"
                  : "border-border/70 text-muted-foreground hover:text-foreground hover:bg-accent/50",
              )}
              onClick={() => onPick(type)}
            >
              {label}
            </button>
          );
        })}
      </div>
    </div>
  );
}

interface IntentGroupProps {
  title: string;
  accent: string;
  intents: { intent: TargetingIntent; label: string }[];
  selected: TargetingIntent | null;
  onPick: (intent: TargetingIntent) => void;
}

function IntentGroup({ title, accent, intents, selected, onPick }: IntentGroupProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <span className={cn("text-[10px] uppercase tracking-wide", accent)}>{title}</span>
      <div className="grid grid-cols-2 gap-1">
        {intents.map(({ intent, label }) => {
          const active = selected === intent;
          return (
            <button
              key={intent}
              className={cn(
                "px-2 py-1 rounded text-[11px] font-medium border text-left",
                active
                  ? "border-primary text-primary bg-primary/10"
                  : "border-border/70 text-muted-foreground hover:text-foreground hover:bg-accent/50",
              )}
              onClick={() => onPick(intent)}
            >
              {label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
