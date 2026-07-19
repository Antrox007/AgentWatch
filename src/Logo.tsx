// Hauptsymbol/Logo — ersetzt das bisherige 🔥-Emoji. EKG-Puls-Linie:
// "live" im selben Sinn wie die Statuspunkte, ohne eine der drei
// Statusfarben zu beanspruchen (grün ist zwar "ready", aber hier als
// eigenständige Markenfarbe verwendet, nicht als Statusaussage).
export function Logo({
  size = 16,
  className,
  glow = true,
}: {
  size?: number;
  className?: string;
  /** false for muted contexts (e.g. the empty state) where an external
   * CSS class (grayscale/opacity) should fully control the look instead —
   * an inline filter would otherwise always win over that class. */
  glow?: boolean;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      className={className}
      style={glow ? { filter: "drop-shadow(0 0 4px color-mix(in srgb, var(--ready) 55%, transparent))" } : undefined}
    >
      <path
        d="M1.5 12.5H6.5L8.5 6.5L12 18.5L14.5 10.5L16.2 12.5H22.5"
        fill="none"
        stroke="var(--ready)"
        strokeWidth="2.2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
