import logoUrl from "@/assets/manaBrew.png";

export function ManaBrewLogo({ size = 48, className }: { size?: number; className?: string }) {
  return (
    <img
      src={logoUrl}
      alt="ManaBrew"
      width={size}
      height={size}
      className={className}
      draggable={false}
    />
  );
}
