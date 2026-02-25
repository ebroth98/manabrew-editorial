import { cn } from "@/lib/utils";

type OverlayVariant = "tap" | "untap" | "choosable" | "pending" | "attacking";

const VARIANT_STYLES: Record<OverlayVariant, { bg: string; border: string; labelBg: string; labelText: string }> = {
  tap: {
    bg: "bg-yellow-400/20",
    border: "border-yellow-400",
    labelBg: "bg-yellow-200/90",
    labelText: "text-yellow-800",
  },
  untap: {
    bg: "bg-cyan-400/20",
    border: "border-cyan-400",
    labelBg: "bg-cyan-200/90",
    labelText: "text-cyan-900",
  },
  choosable: {
    bg: "bg-blue-500/20",
    border: "border-blue-400",
    labelBg: "",
    labelText: "",
  },
  pending: {
    bg: "bg-orange-500/20",
    border: "border-orange-400",
    labelBg: "",
    labelText: "",
  },
  attacking: {
    bg: "bg-red-500/20",
    border: "border-red-500",
    labelBg: "",
    labelText: "",
  },
};

interface CardOverlayButtonProps {
  variant: OverlayVariant;
  onClick: () => void;
  title?: string;
  label?: string;
  /** Stop mousedown propagation (needed in FreeBattlefield to prevent drag) */
  stopMouseDown?: boolean;
}

export function CardOverlayButton({ variant, onClick, title, label, stopMouseDown }: CardOverlayButtonProps) {
  const styles = VARIANT_STYLES[variant];
  return (
    <button
      className={cn(
        "absolute inset-0 z-20 rounded-lg opacity-0 group-hover:opacity-100 border-2 transition-opacity",
        styles.bg,
        styles.border,
        label && "flex items-end justify-center pb-1",
      )}
      onClick={onClick}
      onMouseDown={stopMouseDown ? (e) => e.stopPropagation() : undefined}
      title={title}
    >
      {label && (
        <span className={cn("text-[9px] font-bold px-1 rounded leading-none", styles.labelBg, styles.labelText)}>
          {label}
        </span>
      )}
    </button>
  );
}
