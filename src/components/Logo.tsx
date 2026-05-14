interface Props {
  size?: number;
  className?: string;
  /** Random suffix so multiple instances on the page get unique gradient IDs. */
  idSuffix?: string;
}

/// The Dicto mark — three sound-wave bars over a pastel sky → lavender →
/// blush gradient. Pastel-shifted in v0.3.0 to match the rest of the
/// app palette; the bundled `.icns` / `.png` icons are regenerated to
/// match (see scripts/regen-icons.sh).
export function Logo({ size = 32, className, idSuffix = "default" }: Props) {
  const gradId = `dicto-grad-${idSuffix}`;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 100 100"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-label="Dicto"
      role="img"
    >
      <defs>
        <linearGradient id={gradId} x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#A7C7E7" />
          <stop offset="50%" stopColor="#B5ACE5" />
          <stop offset="100%" stopColor="#F2C6D1" />
        </linearGradient>
      </defs>
      <rect x="0" y="0" width="100" height="100" rx="22" ry="22" fill={`url(#${gradId})`} />
      {/* Three sound-wave bars: short, tall, medium. White stays as-is
          — pastel backdrop means white needs all the contrast it can get. */}
      <rect x="32" y="35" width="8" height="30" rx="4" fill="#ffffff" />
      <rect x="46" y="22" width="8" height="56" rx="4" fill="#ffffff" />
      <rect x="60" y="30" width="8" height="40" rx="4" fill="#ffffff" />
    </svg>
  );
}
