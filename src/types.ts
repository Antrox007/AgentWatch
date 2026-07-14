// Spiegel der Rust-Datenstrukturen (src-tauri/src/model.rs), camelCase.

export type SessionState = "working" | "waiting" | "ready" | "unknown";

export interface Subagent {
  id: string;
  agentType: string;
  state: SessionState;
}

export interface HistoryPoint {
  t: number;
  state: SessionState;
}

export interface SessionInfo {
  sessionId: string;
  pid: number;
  cwd: string;
  projectName: string;
  gitBranch: string | null;
  state: SessionState;
  model: string | null;
  updatedAt: number;
  startedAt: number | null;
  contextTokens: number | null;
  contextWindow: number | null;
  contextUtilization: number | null;
  estimatedCostUsd: number | null;
  idleInferred: boolean;
  subagentCount: number;
  subagents: Subagent[];
  lastText: string | null;
  history: HistoryPoint[];
}

export interface ProjectGroup {
  projectName: string;
  path: string;
  sessions: SessionInfo[];
}

export interface Totals {
  sessions: number;
  working: number;
  waiting: number;
  ready: number;
  agents: number;
}

export interface RateLimits {
  fiveHourPct: number | null;
  sevenDayPct: number | null;
  fiveHourResetsAt: string | null;
  sevenDayResetsAt: string | null;
}

export interface Snapshot {
  generatedAt: number;
  projects: ProjectGroup[];
  totals: Totals;
  rateLimits: RateLimits | null;
}

export interface AppSettings {
  notifyWaiting: boolean;
  notifyReady: boolean;
  notifyContextPressure: boolean;
  notifyNewSubagent: boolean;
  soundEnabled: boolean;
  showCost: boolean;
  autostart: boolean;
  statuslineIntegration: boolean;
  islandEnabled: boolean;
  islandMonitor: string | null;
}

/** Ein Monitor zur Auswahl der Pill-Position. */
export interface MonitorInfo {
  name: string;
  label: string;
  width: number;
  height: number;
  isPrimary: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  notifyWaiting: false,
  notifyReady: false,
  notifyContextPressure: false,
  notifyNewSubagent: false,
  soundEnabled: true,
  showCost: false,
  autostart: false,
  statuslineIntegration: false,
  islandEnabled: true,
  islandMonitor: null,
};
