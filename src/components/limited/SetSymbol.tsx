import { useSetLookup } from "@/stores/useScryfallStore";
import { cn } from "@/lib/utils";

interface SetSymbolProps {
  setCode: string;
  className?: string;
  color?: string;
}

export function SetSymbol({ setCode, className, color }: SetSymbolProps) {
  const setLookup = useSetLookup();
  const svgUri = setCode ? setLookup.get(setCode.toLowerCase())?.icon_svg_uri : undefined;

  if (!svgUri) {
    return (
      <span
        className={cn(
          "inline-flex items-center justify-center font-mono text-[10px] font-bold uppercase",
          className,
        )}
      >
        {setCode.slice(0, 3)}
      </span>
    );
  }

  return (
    <span
      aria-hidden
      className={cn("inline-block shrink-0", className)}
      style={{
        backgroundColor: color ?? "currentColor",
        WebkitMaskImage: `url(${svgUri})`,
        WebkitMaskRepeat: "no-repeat",
        WebkitMaskSize: "contain",
        WebkitMaskPosition: "center",
        maskImage: `url(${svgUri})`,
        maskRepeat: "no-repeat",
        maskSize: "contain",
        maskPosition: "center",
      }}
    />
  );
}
