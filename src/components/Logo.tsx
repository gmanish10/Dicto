interface Props {
  size?: number;
  className?: string;
  /** Random suffix so multiple instances on the page get unique gradient IDs. */
  idSuffix?: string;
}

/// The Dicto mark — three sound-wave bars on a warm charcoal square,
/// with a single amber accent on the tallest bar. Warm-minimal palette
/// introduced in v0.3.1: charcoal `#1F1B18` ground, cream `#FAF6EF`
/// bars, amber `#D4894A` accent. The bundled `.icns` / `.png` icons
/// are regenerated to match via scripts/regen-icons.sh.
export function Logo({ size = 32, className }: Props) {
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
      <rect x="0" y="0" width="100" height="100" rx="22" ry="22" fill="#1F1B18" />
      {/* Three sound-wave bars: short, tall, medium. The tall middle
          bar carries the amber accent — a single warm beat against the
          cream pair. */}
      <rect x="32" y="35" width="8" height="30" rx="4" fill="#FAF6EF" />
      <rect x="46" y="22" width="8" height="56" rx="4" fill="#D4894A" />
      <rect x="60" y="30" width="8" height="40" rx="4" fill="#FAF6EF" />
    </svg>
  );
}
