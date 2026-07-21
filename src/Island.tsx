import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { getSnapshot, onSnapshot, positionIsland, toggleDashboardTop } from "./api";
import type { SessionInfo, SessionState, Snapshot } from "./types";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { StatusIcon } from "./StatusIcon";
import "./App.css";

// Maximale Anzahl einzelner Punkte, bevor auf "+N" zusammengefasst wird.
const MAX_DOTS = 12;
// Maximale Anzahl gleichzeitig angezeigter Stichpunkte.
const MAX_EVENTS = 4;
// Wie lange ein Ereignis-Stichpunkt sichtbar bleibt, bevor die Pill wieder zuklappt.
const EVENT_TTL_MS = 4500;
// Dauer der Ausblend-Animation, bevor ein abgelaufener Eintrag entfernt wird.
// MUSS mit der CSS-Animation `islandEventOut` in App.css uebereinstimmen.
const EVENT_LEAVE_MS = 280;

// Ein Punkt in der Pill: Status + Anzahl der Subagents genau dieser Session.
interface DotInfo {
  state: SessionState;
  agents: number;
}

// Demo-Daten fuer die reine Browser-Vorschau (kein Tauri-Runtime).
const DEMO_DOTS: DotInfo[] = [
  { state: "working", agents: 2 },
  { state: "working", agents: 0 },
  { state: "waiting", agents: 1 },
  { state: "ready", agents: 0 },
];

// Heartbeat-Intervall: das Island-Fenster ist randlos, transparent und nie
// fokussiert — WebView2 drosselt solche Hintergrundfenster, wodurch gepushte
// `snapshot`-Events ausbleiben koennen und die Pill auf einem veralteten Stand
// einfriert (z.B. laengst beendete "wartet"-Sessions). Als Selbstheilung holt die
// Pill den Snapshot zusaetzlich aktiv per `invoke` ab (IPC, nicht von der
// Event-Drosselung betroffen), sodass sie nie laenger als dieses Intervall driftet.
const SNAPSHOT_POLL_MS = 3000;

interface IslandEvent {
  key: string;
  kind: "ready" | "waiting";
  label: string;
  /** Epoch-ms, ab wann der Eintrag auszublenden beginnt. */
  expiresAt: number;
  /** Epoch-ms, ab wann die Ausblend-Animation laeuft (gesetzt, sobald abgelaufen). */
  leavingAt?: number;
}

// Niemals ablaufender Zeitstempel (fuer die statischen Demo-Eintraege).
const NEVER = Number.MAX_SAFE_INTEGER;

function flattenSessions(snapshot: Snapshot | null): SessionInfo[] {
  if (!snapshot) return [];
  return snapshot.projects.flatMap((p) => p.sessions);
}

export default function Island() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [demo, setDemo] = useState(false);
  const [events, setEvents] = useState<IslandEvent[]>([]);
  const [flashReady, setFlashReady] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const prevStates = useRef<Map<string, SessionState>>(new Map());
  // Zeitstempel (Backend-`generatedAt`) des zuletzt uebernommenen Snapshots.
  // Push-Events (ueber den gedrosselten Tauri-Event-Kanal, siehe Heartbeat-
  // Kommentar unten) und der ungedrosselte Poll laufen als zwei unabhaengige
  // Kanaele nebeneinander her und koennen daher in falscher Reihenfolge
  // eintreffen: ein aelterer, aber verspaeteter Push kann kurzzeitig einen
  // bereits ueberholten Zustand zurueckschreiben. Die darunterliegende
  // Uebergangserkennung wuerde denselben Wechsel (z.B. "fertig") dann ein
  // zweites Mal melden. Guard: nur echt neuere Snapshots uebernehmen.
  const lastGeneratedAt = useRef(0);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    const applySnapshot = (snap: Snapshot) => {
      if (snap.generatedAt < lastGeneratedAt.current) return;
      lastGeneratedAt.current = snap.generatedAt;
      setSnapshot(snap);
    };

    getSnapshot()
      .then(applySnapshot)
      .catch(() => {
        // Browser-Vorschau: Demo-Daten inkl. zwei Beispiel-Ereignissen (Pill aufgeklappt).
        setDemo(true);
        setEvents([
          { key: "demo-r", kind: "ready", label: "todo-app: fertig", expiresAt: NEVER },
          {
            key: "demo-w",
            kind: "waiting",
            label: "api-server: wartet auf Eingabe",
            expiresAt: NEVER,
          },
        ]);
      });
    onSnapshot(applySnapshot)
      .then((fn) => (unlisten = fn))
      .catch(() => {});

    // Heartbeat: aktiv pollen, falls gepushte Events (Hintergrund-Drosselung)
    // ausbleiben. Erfolgreiches Abholen verlaesst zugleich den Demo-Modus.
    const poll = setInterval(() => {
      getSnapshot()
        .then((snap) => {
          applySnapshot(snap);
          setDemo(false);
        })
        .catch(() => {});
    }, SNAPSHOT_POLL_MS);

    return () => {
      unlisten?.();
      clearInterval(poll);
    };
  }, []);

  // Zustandswechsel erkennen -> Ereignis-Stichpunkte (fertig/wartet) + gruener Blitz.
  useEffect(() => {
    if (demo) return;
    const sess = flattenSessions(snapshot);
    const fresh: { kind: "ready" | "waiting"; label: string }[] = [];
    let newlyReady = false;
    for (const s of sess) {
      const prev = prevStates.current.get(s.sessionId);
      if (prev === s.state) continue;
      if (s.state === "ready") {
        fresh.push({ kind: "ready", label: `${s.projectName}: fertig` });
        newlyReady = true;
      } else if (s.state === "waiting") {
        fresh.push({ kind: "waiting", label: `${s.projectName}: wartet auf Eingabe` });
      }
    }
    const hadPrev = prevStates.current.size > 0;
    const next = new Map<string, SessionState>();
    sess.forEach((s) => next.set(s.sessionId, s.state));
    prevStates.current = next;

    // Beim allerersten Snapshot nicht alle aktuellen Sessions als "Ereignis" melden.
    if (!hadPrev || fresh.length === 0) return;

    const now = Date.now();
    const stamped: IslandEvent[] = fresh.map((e, i) => ({
      ...e,
      key: `${now}-${i}-${e.label}`,
      expiresAt: now + EVENT_TTL_MS,
    }));
    // Dedup: gleiche Meldung ersetzen (Timer auffrischen), statt zu stapeln.
    const freshLabels = new Set(stamped.map((e) => e.label));
    setEvents((prev) =>
      [...stamped, ...prev.filter((e) => !freshLabels.has(e.label))].slice(0, MAX_EVENTS),
    );
    if (newlyReady) {
      setFlashReady(true);
      setTimeout(() => setFlashReady(false), 2000);
    }
  }, [snapshot, demo]);

  // Abgelaufene Eintraege ausblenden + entfernen — bewusst ein EIGENES Intervall,
  // NICHT an den Effect-Cleanup gekoppelt (sonst wuerden die Timer bei jedem
  // Snapshot wieder geloescht und die Eintraege blieben fuer immer stehen).
  // Zweistufig: erst `leavingAt` setzen (startet die CSS-Ausblendung), dann nach
  // EVENT_LEAVE_MS endgueltig entfernen — so klappt die Pill nicht hart zu.
  useEffect(() => {
    if (demo) return;
    const id = setInterval(() => {
      const now = Date.now();
      setEvents((prev) => {
        let changed = false;
        const next: IslandEvent[] = [];
        for (const e of prev) {
          if (e.leavingAt != null) {
            // In der Ausblendung — erst nach Ablauf der Animation entfernen.
            if (now - e.leavingAt >= EVENT_LEAVE_MS) changed = true;
            else next.push(e);
          } else if (e.expiresAt <= now) {
            // TTL erreicht — Ausblendung starten.
            next.push({ ...e, leavingAt: now });
            changed = true;
          } else {
            next.push(e);
          }
        }
        return changed ? next : prev;
      });
    }, 200);
    return () => clearInterval(id);
  }, [demo]);

  const sessions = demo ? [] : flattenSessions(snapshot);
  const dots: DotInfo[] = demo
    ? DEMO_DOTS
    : sessions.map((s) => ({ state: s.state, agents: s.subagentCount ?? 0 }));
  const working = dots.filter((d) => d.state === "working").length;
  const waiting = dots.filter((d) => d.state === "waiting").length;
  const ready = dots.filter((d) => d.state === "ready").length;
  const shown = dots.slice(0, MAX_DOTS);
  const more = dots.length - shown.length;
  const expanded = events.length > 0;
  // Aenderung der Agent-Zahlen kann die Pill-Breite beeinflussen (mehrstellige
  // Zahl breiter als der Punkt) — als Signal fuer die Groessenmessung mitfuehren.
  const agentsKey = dots.map((d) => d.agents).join(",");

  // Nach jedem Render die gemessene Groesse (Breite x Hoehe) ans Backend melden,
  // damit das Fenster exakt darauf zugeschnitten + oben zentriert wird. Gemessen
  // wird der Stage-Wrapper INKL. transparentem Bleed-Rand (CSS `--bleed`), damit
  // aeussere Glows / Hover-Lift / Aufklapp-Overshoot Platz haben und nicht am
  // Fensterrand abgeschnitten werden.
  useLayoutEffect(() => {
    if (!ref.current) return;
    const rect = ref.current.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      positionIsland(Math.ceil(rect.width), Math.ceil(rect.height)).catch(() => {});
    }
  }, [dots.length, working, waiting, ready, more, agentsKey, events]);

  // Dominanter Status faerbt die kontinuierliche Aura (auch ohne Ereignis):
  // wartet (dringend) > arbeitet > bereit. Ohne Sessions: keine Aura.
  const dominant =
    waiting > 0 ? "waiting" : working > 0 ? "working" : ready > 0 ? "ready" : null;

  const stageClass = ["island-stage", dominant ? `island-dom-${dominant}` : ""]
    .filter(Boolean)
    .join(" ");

  const pillClass = [
    "island",
    expanded ? "island-expanded" : "",
    waiting > 0 ? "island-attn-waiting" : "",
    flashReady ? "island-flash-ready" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    // Stage = transparenter Bleed-Rand um die Pill (CSS `--bleed`). Hier sitzt der
    // ref fuer die Groessenmessung; Klick/Hover/Animationen liegen auf der Pill.
    <div className={stageClass} ref={ref}>
      {/* Kontinuierliche Status-Aura HINTER der Pill (im Bleed, daher nicht vom
          overflow:hidden der Pill abgeschnitten). Farbe/Tempo per Dominant-Klasse. */}
      <div className="island-glow" aria-hidden="true" />
      <div
        className={pillClass}
        onClick={() => toggleDashboardTop().catch(() => {})}
        title="AgentWatch — klicken zum Öffnen/Schließen"
      >
        <div className="island-row">
          {dots.length === 0 ? (
            <span className="island-idle">keine Sessions</span>
          ) : (
            <span className="island-dots">
              {shown.map((d, i) => (
                // Spalte je Session: Statuspunkt oben, Anzahl der Subagents dieser
                // Session direkt darunter (0 = gedimmt, >0 = hervorgehoben).
                <span className="island-dotcol" key={i}>
                  <StatusIcon
                    state={d.state}
                    size={11}
                    className="island-status-icon"
                    unknownClassName="island-dot island-dot-unknown"
                  />
                  <span
                    className={`island-dot-agents${d.agents > 0 ? " has-agents" : ""}`}
                  >
                    {d.agents}
                  </span>
                </span>
              ))}
              {more > 0 && <span className="island-more">+{more}</span>}
            </span>
          )}
        </div>
        {expanded && (
          <div className="island-events">
            {events.map((e, i) => (
              <div
                className={`island-event${e.leavingAt != null ? " island-event-leaving" : ""}`}
                key={e.key}
                style={{ "--i": i } as CSSProperties}
              >
                <StatusIcon
                  state={e.kind}
                  size={13}
                  className="island-status-icon"
                  unknownClassName="island-dot island-dot-unknown"
                />
                <span className="island-event-label">{e.label}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
