import type { CSSProperties, ReactNode } from "react";
import { useState } from "react";

export interface OrbitBadge {
  id: string;
  /** Icon node rendered inside the badge chip. */
  icon: ReactNode;
  /** Accessible label shown in the tooltip. */
  label: string;
  /** Optional numeric count rendered next to the icon. */
  count?: number;
  /** Badge chip colour. Defaults to neutral tint. */
  color?: string;
  /** Callback invoked when the chip is clicked. Makes the chip a button. */
  onClick?: () => void;
}

type Slot = "tr" | "tl" | "mr" | "ml";

// Bottom slots are intentionally omitted — the centered life chip lives
// at `-bottom-2, left-1/2` and would overlap any bottom-corner badge.
// Priority order fills top corners first, then drops to mid-sides.
const SLOT_ORDER: Slot[] = ["tr", "tl", "mr", "ml"];

const SLOT_STYLE: Record<Slot, CSSProperties> = {
  tr: { top: -6, right: -6 },
  tl: { top: -6, left: -6 },
  mr: { top: "50%", right: -8, transform: "translateY(-50%)" },
  ml: { top: "50%", left: -8, transform: "translateY(-50%)" },
};

const MAX_VISIBLE = 3;

interface BadgeChipProps {
  badge: OrbitBadge;
  style: CSSProperties;
}

function BadgeChip({ badge, style }: BadgeChipProps) {
  const chipStyle: CSSProperties = {
    ...style,
    backgroundColor: badge.color ?? "rgba(0, 0, 0, 0.85)",
  };
  const content = (
    <>
      <span className="inline-flex items-center justify-center">{badge.icon}</span>
      {badge.count !== undefined && (
        <span className="text-[10px] font-bold leading-none tabular-nums">{badge.count}</span>
      )}
      <span className="pointer-events-none absolute left-1/2 top-full z-40 mt-1 -translate-x-1/2 whitespace-nowrap rounded bg-black/85 px-1.5 py-0.5 text-[10px] font-semibold text-white opacity-0 transition-opacity duration-150 delay-0 group-hover/badge:delay-300 group-hover/badge:opacity-100">
        {badge.label}
      </span>
    </>
  );
  const cls =
    "group/badge absolute z-20 inline-flex items-center gap-0.5 rounded-full px-1 py-0.5 text-white shadow ring-1 ring-black/40 backdrop-blur-sm";
  if (badge.onClick) {
    return (
      <button type="button" className={cls} style={chipStyle} onClick={badge.onClick}>
        {content}
      </button>
    );
  }
  return (
    <span className={cls} style={chipStyle}>
      {content}
    </span>
  );
}

interface OverflowChipProps {
  extras: OrbitBadge[];
  style: CSSProperties;
}

function OverflowChip({ extras, style }: OverflowChipProps) {
  const [open, setOpen] = useState(false);
  const chipStyle: CSSProperties = {
    ...style,
    backgroundColor: "rgba(0, 0, 0, 0.85)",
  };
  return (
    <button
      type="button"
      className="group/badge absolute z-20 inline-flex items-center gap-0.5 rounded-full px-1.5 py-0.5 text-[10px] font-bold text-white shadow ring-1 ring-black/40 backdrop-blur-sm"
      style={chipStyle}
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onFocus={() => setOpen(true)}
      onBlur={() => setOpen(false)}
    >
      +{extras.length}
      {open && (
        <span className="pointer-events-none absolute left-1/2 top-full z-40 mt-1 flex -translate-x-1/2 flex-col gap-0.5 whitespace-nowrap rounded bg-black/90 px-2 py-1 text-[10px] font-semibold text-white shadow-lg">
          {extras.map((b) => (
            <span key={b.id} className="inline-flex items-center gap-1">
              <span className="inline-flex items-center">{b.icon}</span>
              <span>{b.label}</span>
              {b.count !== undefined && <span className="tabular-nums">×{b.count}</span>}
            </span>
          ))}
        </span>
      )}
    </button>
  );
}

export function BadgeOrbit({
  badges,
  avatarSize: _avatarSize,
}: {
  badges: OrbitBadge[];
  avatarSize: number;
}) {
  if (badges.length === 0) return null;

  const visible = badges.length <= SLOT_ORDER.length ? badges : badges.slice(0, MAX_VISIBLE);
  const extras = badges.length <= SLOT_ORDER.length ? [] : badges.slice(MAX_VISIBLE);

  return (
    <>
      {visible.map((badge, i) => (
        <BadgeChip key={badge.id} badge={badge} style={SLOT_STYLE[SLOT_ORDER[i]!]} />
      ))}
      {extras.length > 0 && (
        <OverflowChip extras={extras} style={SLOT_STYLE[SLOT_ORDER[MAX_VISIBLE]!]} />
      )}
    </>
  );
}
