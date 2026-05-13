interface Props {
  size?: number;
  className?: string;
  /** Random suffix so multiple instances on the page get unique gradient IDs. */
  idSuffix?: string;
}

/// The Dicto mark — three sound-wave bars over a blue → violet → pink gradient,
/// matching the bundled .icns / .png icons.
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
          <stop offset="0%" stopColor="#5b8def" />
          <stop offset="50%" stopColor="#8b5cf6" />
          <stop offset="100%" stopColor="#ec4899" />
        </linearGradient>
      </defs>
      <rect x="0" y="0" width="100" height="100" rx="22" ry="22" fill={`url(#${gradId})`} />
      {/* Three sound-wave bars: short, tall, medium */}
      <rect x="32" y="35" width="8" height="30" rx="4" fill="#ffffff" />
      <rect x="46" y="22" width="8" height="56" rx="4" fill="#ffffff" />
      <rect x="60" y="30" width="8" height="40" rx="4" fill="#ffffff" />
    </svg>
  );
}
