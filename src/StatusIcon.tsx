import { useId } from "react";
import type { SessionState } from "./types";

// Apple/SF-Symbols-style Glyphen statt reiner Farbpunkte: Zahnrad (working),
// Haekchen (ready), Ausrufezeichen (waiting). "unknown" bleibt der schlichte
// Punkt (Fallback-Zustand, kein eigenes Icon vorgesehen).
//
// Bewusst KEIN Kachel-/Squircle-Container (das wurde ausprobiert und verworfen
// — wirkte wie ein fremdes Homescreen-Icon auf der Seite). Reine Glyphen mit
// glatten Kurven (kein Pixel-Grid) plus dezentem Glow, im selben Stil wie
// zuvor die reinen Punkte.

const TOOTH_ANGLES = [0, 45, 90, 135, 180, 225, 270, 315];

function glow(color: string): string {
  return `drop-shadow(0 0 4px color-mix(in srgb, ${color} 55%, transparent))`;
}

function GearIcon({ size }: { size: number }) {
  // Echtes transparentes Loch per SVG-Mask (nicht einfarbig ausgefuellt) —
  // funktioniert unabhaengig vom Hintergrund dahinter. Eindeutige id pro
  // Instanz noetig, da mehrere Zahnraeder gleichzeitig auf der Seite stehen
  // koennen (Dashboard + Pill) und SVG-mask-ids sonst kollidieren.
  const maskId = useId();
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      style={{
        filter: glow("var(--working)"),
        animation: "icon-spin 3.5s linear infinite",
        transformOrigin: "50% 50%",
      }}
    >
      <defs>
        <mask id={maskId} maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24">
          <rect x="0" y="0" width="24" height="24" fill="white" />
          <circle cx="12" cy="12" r="3.1" fill="black" />
        </mask>
      </defs>
      <g fill="var(--working)" mask={`url(#${maskId})`}>
        {TOOTH_ANGLES.map((angle) => (
          <rect
            key={angle}
            x="10.6"
            y="0.6"
            width="2.8"
            height="4.6"
            rx="1.3"
            transform={`rotate(${angle} 12 12)`}
          />
        ))}
        <circle cx="12" cy="12" r="7.4" />
      </g>
    </svg>
  );
}

function CheckIcon({ size }: { size: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      style={{
        filter: glow("var(--ready)"),
        animation: "apple-pop-in 0.5s cubic-bezier(0.34, 1.56, 0.64, 1) both",
      }}
    >
      <path
        d="M4.5 12.5L9.5 17.5L19.5 6"
        fill="none"
        stroke="var(--ready)"
        strokeWidth="3"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function ExclaimIcon({ size }: { size: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      style={{
        filter: glow("var(--waiting)"),
        animation: "apple-nudge 2.2s ease-in-out infinite",
      }}
    >
      <rect x="10.3" y="3" width="3.4" height="12.5" rx="1.7" fill="var(--waiting)" />
      <circle cx="12" cy="19.3" r="1.9" fill="var(--waiting)" />
    </svg>
  );
}

export function StatusIcon({
  state,
  size = 14,
  className,
  unknownClassName,
  title,
}: {
  state: SessionState;
  size?: number;
  /** Klasse fuer den Icon-Wrapper (Ausrichtung/Abstand, keine feste Groesse). */
  className?: string;
  /**
   * Klasse fuer den Fallback-Punkt bei state "unknown" — muss die
   * kontextpassende bestehende Punkt-Klasse sein (z.B. "dot dot-unknown"
   * im Dashboard oder "island-dot island-dot-unknown" in der Pill), da
   * dort weiterhin ein schlichter Punkt statt eines Icons steht.
   */
  unknownClassName: string;
  title?: string;
}) {
  if (state === "unknown") {
    return <span className={unknownClassName} title={title} />;
  }
  const Icon = state === "working" ? GearIcon : state === "waiting" ? ExclaimIcon : CheckIcon;
  return (
    <span
      className={className}
      title={title}
      style={{ display: "inline-flex", lineHeight: 0 }}
    >
      <Icon size={size} />
    </span>
  );
}
