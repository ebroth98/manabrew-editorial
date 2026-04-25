/**
 * OpenMagic logo as an inline SVG component.
 * Background rect uses the current theme's background color.
 */
export function OpenMagicLogo({ size = 48, className }: { size?: number; className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 512 512"
      width={size}
      height={size}
      className={className}
    >
      <defs>
        <linearGradient id="om-grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#A866E5" />
          <stop offset="30%" stopColor="#A866E5" />
          <stop offset="100%" stopColor="#E5B833" />
        </linearGradient>
        <filter id="subtle-glow" x="-30%" y="-30%" width="160%" height="160%">
          <feGaussianBlur stdDeviation="6" result="blur" />
          <feComponentTransfer in="blur" result="faint-blur">
            <feFuncA type="linear" slope="1.0" />
          </feComponentTransfer>
          <feMerge>
            <feMergeNode in="faint-blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      {/* Background — uses theme background via CSS variable */}
      <rect width="512" height="512" rx="112" fill="var(--background)" />

      {/* Outer ring */}
      <circle
        cx="256"
        cy="256"
        r="180"
        fill="none"
        stroke="var(--border)"
        strokeWidth="2"
        strokeOpacity="0.3"
        strokeDasharray="12 12"
      />

      {/* O + M mark */}
      <g transform="translate(156, 156)">
        <circle
          cx="100"
          cy="100"
          r="80"
          fill="none"
          stroke="url(#om-grad)"
          strokeWidth="16"
          strokeLinecap="round"
          filter="url(#subtle-glow)"
        />
        <path
          d="M 60 135 L 60 65 L 100 105 L 140 65 L 140 135"
          fill="none"
          stroke="var(--foreground)"
          strokeWidth="16"
          strokeLinecap="round"
          strokeLinejoin="round"
          filter="url(#subtle-glow)"
        />
      </g>

      {/* Mana orbs */}
      <circle cx="256" cy="76" r="28" fill="#D1D5DB" opacity="0.4" filter="url(#subtle-glow)" />
      <circle cx="256" cy="76" r="24" fill="#F9FAFB" filter="url(#subtle-glow)" />
      <circle cx="427" cy="200" r="24" fill="#3B82F6" filter="url(#subtle-glow)" />
      <circle cx="362" cy="402" r="24" fill="#374151" filter="url(#subtle-glow)" />
      <circle cx="150" cy="402" r="24" fill="#EF4444" filter="url(#subtle-glow)" />
      <circle cx="85" cy="200" r="24" fill="#10B981" filter="url(#subtle-glow)" />
    </svg>
  );
}
