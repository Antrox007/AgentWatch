import { useEffect, useState } from "react";
import { getSettings, getSnapshot, hideDashboard, onSnapshot, saveSettings } from "./api";
import SettingsPanel from "./SettingsPanel";
import {
  DEFAULT_SETTINGS,
  type AppSettings,
  type HistoryPoint,
  type ProjectGroup,
  type RateLimits,
  type SessionInfo,
  type SessionState,
  type Snapshot,
} from "./types";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { fmtTokens, fmtCost, shortModel } from "./format";
import { StatusIcon } from "./StatusIcon";
import { Logo } from "./Logo";
import { UpdateBanner } from "./UpdateBanner";
import { useUpdater } from "./useUpdater";
import "./App.css";

const STATE_LABEL: Record<SessionState, string> = {
  working: "arbeitet",
  waiting: "wartet auf Eingabe",
  ready: "bereit",
  unknown: "unbekannt",
};

// Hilfsfunktion fuer Demo-Historie.
function hist(states: SessionState[]): HistoryPoint[] {
  return states.map((state, i) => ({ t: i, state }));
}

// Demo-Daten fuer die reine Browser-Vorschau (kein Tauri-Runtime vorhanden).
const DEMO: Snapshot = {
  generatedAt: Date.now(),
  totals: { sessions: 3, working: 1, waiting: 1, ready: 1, agents: 2 },
  rateLimits: {
    fiveHourPct: 42,
    sevenDayPct: 18,
    fiveHourResetsAt: null,
    sevenDayResetsAt: null,
  },
  projects: [
    {
      projectName: "todo-app",
      path: "C:\\projects\\todo-app",
      sessions: [
        {
          sessionId: "demo-1",
          pid: 30592,
          cwd: "C:\\projects\\todo-app",
          projectName: "todo-app",
          gitBranch: "main",
          state: "working",
          model: "claude-opus-4-8",
          updatedAt: Date.now(),
          startedAt: Date.now() - 600000,
          contextTokens: 47800,
          contextWindow: 200000,
          contextUtilization: 0.239,
          estimatedCostUsd: 0.42,
          idleInferred: false,
          subagentCount: 2,
          subagents: [
            { id: "a1", agentType: "Explore", state: "working" },
            { id: "a2", agentType: "general-purpose", state: "working" },
          ],
          lastText: null,
          history: hist([
            "ready",
            "working",
            "working",
            "waiting",
            "working",
            "working",
            "working",
          ]),
        },
      ],
    },
    {
      projectName: "api-server",
      path: "C:\\projects\\api-server",
      sessions: [
        {
          sessionId: "demo-2",
          pid: 18484,
          cwd: "C:\\projects\\api-server",
          projectName: "api-server",
          gitBranch: "master",
          state: "waiting",
          model: "claude-sonnet-4-6",
          updatedAt: Date.now(),
          startedAt: Date.now() - 1200000,
          contextTokens: 172400,
          contextWindow: 200000,
          contextUtilization: 0.862,
          estimatedCostUsd: 1.93,
          idleInferred: false,
          subagentCount: 0,
          subagents: [],
          lastText:
            "Soll ich die Migration auf master pushen oder erst einen Branch anlegen?",
          history: hist([
            "working",
            "working",
            "working",
            "working",
            "waiting",
            "waiting",
          ]),
        },
        {
          sessionId: "demo-3",
          pid: 3412,
          cwd: "C:\\projects\\api-server",
          projectName: "api-server",
          gitBranch: "feature/timesavings",
          state: "ready",
          model: "claude-haiku-4-5",
          updatedAt: Date.now(),
          startedAt: Date.now() - 200000,
          contextTokens: 12300,
          contextWindow: 200000,
          contextUtilization: 0.0615,
          estimatedCostUsd: 0.08,
          idleInferred: false,
          subagentCount: 0,
          subagents: [],
          lastText: null,
          history: hist(["working", "working", "ready", "ready"]),
        },
      ],
    },
  ],
};

function StatusDot({ state }: { state: SessionState }) {
  return (
    <StatusIcon
      state={state}
      size={14}
      title={STATE_LABEL[state]}
      className="status-dot-dashboard"
      unknownClassName="dot dot-unknown status-dot-dashboard"
    />
  );
}

function ContextBar({ session }: { session: SessionInfo }) {
  if (session.contextUtilization == null) return null;
  const pct = Math.round(session.contextUtilization * 100);
  const level =
    session.contextUtilization >= 0.85
      ? "crit"
      : session.contextUtilization >= 0.7
        ? "warn"
        : "ok";
  return (
    <div
      className="ctxbar"
      title={`Kontext ${pct}% (${fmtTokens(session.contextTokens)} / ${fmtTokens(session.contextWindow)})`}
    >
      <div className={`ctxbar-fill ${level}`} style={{ width: `${pct}%` }} />
      <span className="ctxbar-label">{pct}%</span>
    </div>
  );
}

function RateLimitChips({ limits }: { limits: RateLimits }) {
  const chip = (label: string, pct: number | null) => {
    if (pct == null) return null;
    const level = pct >= 90 ? "crit" : pct >= 70 ? "warn" : "ok";
    return (
      <span
        className={`quota quota-${level}`}
        title={`${label}-Fenster: ${pct.toFixed(0)}% verbraucht`}
      >
        {label} {pct.toFixed(0)}%
      </span>
    );
  };
  if (limits.fiveHourPct == null && limits.sevenDayPct == null) return null;
  return (
    <div className="quotas">
      <span className="quota-caption">Limit</span>
      {chip("5h", limits.fiveHourPct)}
      {chip("7d", limits.sevenDayPct)}
    </div>
  );
}

function HistoryBar({ history }: { history: HistoryPoint[] }) {
  if (!history || history.length < 2) return null;
  return (
    <div className="histbar" title="Zustands-Verlauf (neueste rechts)">
      {history.map((point, i) => (
        <span key={i} className={`histcell hist-${point.state}`} />
      ))}
    </div>
  );
}

function SubagentTree({ subagents }: { subagents: SessionInfo["subagents"] }) {
  return (
    <div className="subagents">
      {subagents.map((sa) => (
        <div className="subagent" key={sa.id}>
          <span className={`dot-sm dot-${sa.state}`} />
          <span className="subagent-type">{sa.agentType}</span>
          <span className="subagent-state">{STATE_LABEL[sa.state]}</span>
        </div>
      ))}
    </div>
  );
}

function SessionRow({
  session,
  showCost,
}: {
  session: SessionInfo;
  showCost: boolean;
}) {
  const [open, setOpen] = useState(false);
  const hasSubs = session.subagents.length > 0;

  return (
    <div className="session">
      <StatusDot state={session.state} />
      <div className="session-main">
        <div className="session-top">
          <span className="state-text">{STATE_LABEL[session.state]}</span>
          {session.gitBranch && (
            <span className="branch" title="Git-Branch">
              ⎇ {session.gitBranch}
            </span>
          )}
          {session.model && (
            <span className="model">{shortModel(session.model)}</span>
          )}
          {hasSubs && (
            <button
              className="agents"
              onClick={() => setOpen((o) => !o)}
              title="Subagents anzeigen"
            >
              ⛓ {session.subagentCount} {open ? "▾" : "▸"}
            </button>
          )}
        </div>
        {session.state === "waiting" && session.lastText && (
          <div className="waiting-text">{session.lastText}</div>
        )}
        {open && hasSubs && <SubagentTree subagents={session.subagents} />}
        <div className="session-bottom">
          <ContextBar session={session} />
          <span className="tokens">{fmtTokens(session.contextTokens)} Tok.</span>
          {showCost && session.estimatedCostUsd != null && (
            <span
              className="cost"
              title="Geschätzte Gesamtkosten dieser Session (kumulativ über alle Turns, Listenpreise)"
            >
              {fmtCost(session.estimatedCostUsd)}
            </span>
          )}
          <span className="pid">PID {session.pid}</span>
        </div>
        <HistoryBar history={session.history} />
      </div>
    </div>
  );
}

function ProjectGroupView({
  group,
  showCost,
}: {
  group: ProjectGroup;
  showCost: boolean;
}) {
  const n = group.sessions.length;
  return (
    <section className="group">
      <header className="group-head">
        <span className="group-name">{group.projectName}</span>
        <span className="group-count">
          {n} {n === 1 ? "Session" : "Sessions"}
        </span>
      </header>
      {group.sessions.map((session) => (
        <SessionRow
          key={session.sessionId}
          session={session}
          showCost={showCost}
        />
      ))}
    </section>
  );
}

export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [view, setView] = useState<"dashboard" | "settings">("dashboard");
  const [demo, setDemo] = useState(false);
  // Steuert die Content-Einblendung. Wird bei jedem Fenster-Fokus neu ausgeloest:
  // Das Dashboard ist ein Flyout, das bei Pill-/Tray-Klick gezeigt + fokussiert
  // wird (und bei Fokusverlust verschwindet). So spielt die Einblendung bei jedem
  // Oeffnen erneut ab, obwohl der React-Baum erhalten bleibt.
  const [appear, setAppear] = useState(true);
  const { state: updateState, install: installUpdate, dismiss: dismissUpdate } = useUpdater();

  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;

    // Live-Listener IMMER zuerst anhaengen: Der Watcher sendet alle ~2s einen
    // Snapshot; der erste echte Snapshot ersetzt evtl. gesetzte Demo-Daten und
    // heilt so jeden transienten Startup-Fehler des initialen Commands.
    onSnapshot((snap) => {
      if (cancelled) return;
      setSnapshot(snap);
      setDemo(false);
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {});

    // Initialer Snapshot — nur dessen Scheitern aktiviert (vorlaeufig) den Demo-Modus.
    getSnapshot()
      .then((snap) => {
        if (!cancelled) {
          setSnapshot(snap);
          setDemo(false);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setSnapshot(DEMO);
          setDemo(true);
        }
      });

    // Einstellungen unabhaengig laden — ein Fehler hier beeinflusst die Daten nicht.
    getSettings()
      .then((s) => {
        if (!cancelled) setSettings(s);
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, []);

  // Esc schließt das Dashboard-Flyout (Pendant zum Klick daneben / Fokusverlust).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") hideDashboard().catch(() => {});
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Content-Einblendung bei jedem Oeffnen des Flyouts neu starten: Klasse kurz
  // entfernen und im naechsten Frame wieder setzen, damit die CSS-Animation
  // erneut abspielt (sonst liefe sie nur beim allerersten Mount).
  useEffect(() => {
    const replay = () => {
      setAppear(false);
      requestAnimationFrame(() => requestAnimationFrame(() => setAppear(true)));
    };
    window.addEventListener("focus", replay);
    return () => window.removeEventListener("focus", replay);
  }, []);

  const handleSettingsChange = (next: AppSettings) => {
    setSettings(next);
    saveSettings(next).catch(() => {});
  };

  const totals = snapshot?.totals;

  if (view === "settings") {
    return (
      <div className={`app${appear ? " app-appear" : ""}`}>
        <SettingsPanel
          settings={settings}
          onChange={handleSettingsChange}
          onClose={() => setView("dashboard")}
        />
      </div>
    );
  }

  return (
    <div className={`app${appear ? " app-appear" : ""}`}>
      <header className="header">
        <div className="brand">
          <Logo size={17} className="flame" />
          <span className="title">AgentWatch</span>
          {demo && <span className="demo-badge">Demo</span>}
          <button
            className="gear-btn"
            onClick={() => setView("settings")}
            title="Einstellungen"
          >
            ⚙
          </button>
        </div>
        {totals && (
          <div className="totals">
            <span className="pill pill-working">{totals.working} aktiv</span>
            <span className="pill pill-waiting">{totals.waiting} warten</span>
            <span className="pill pill-ready">{totals.ready} bereit</span>
            <span className="pill pill-agents">{totals.agents} Agents</span>
          </div>
        )}
        {snapshot?.rateLimits && <RateLimitChips limits={snapshot.rateLimits} />}
      </header>

      <UpdateBanner state={updateState} onInstall={installUpdate} onDismiss={dismissUpdate} />

      <main className="content">
        {!snapshot && <div className="empty">Lade …</div>}
        {snapshot && snapshot.projects.length === 0 && (
          <div className="empty">
            <Logo size={34} className="empty-flame" glow={false} />
            <div className="empty-title">Keine aktiven Sessions</div>
            <div className="empty-sub">
              Starte eine Claude-Code-Session, um sie hier zu sehen.
            </div>
          </div>
        )}
        {snapshot &&
          snapshot.projects.map((group) => (
            <ProjectGroupView
              key={group.path}
              group={group}
              showCost={settings.showCost}
            />
          ))}
      </main>
    </div>
  );
}
