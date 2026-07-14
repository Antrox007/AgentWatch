import { useEffect, useState } from "react";
import { listMonitors } from "./api";
import type { AppSettings, MonitorInfo } from "./types";

type ToggleKey = keyof Pick<
  AppSettings,
  | "notifyWaiting"
  | "notifyReady"
  | "notifyContextPressure"
  | "notifyNewSubagent"
  | "soundEnabled"
  | "showCost"
  | "autostart"
>;

const NOTIFY_TOGGLES: { key: ToggleKey; label: string }[] = [
  { key: "notifyWaiting", label: "Agent wartet auf Eingabe" },
  { key: "notifyReady", label: "Agent fertig" },
  { key: "notifyContextPressure", label: "Context-Pressure (ab 80%)" },
  { key: "notifyNewSubagent", label: "Neuer Subagent gestartet" },
];

const GENERAL_TOGGLES: { key: ToggleKey; label: string }[] = [
  { key: "soundEnabled", label: "Sound bei Benachrichtigungen" },
  { key: "showCost", label: "Geschätzte Session-Kosten (USD) anzeigen" },
  { key: "autostart", label: "Beim Windows-Login automatisch starten" },
];

function ToggleRow({
  label,
  checked,
  onToggle,
}: {
  label: string;
  checked: boolean;
  onToggle: () => void;
}) {
  return (
    <label className="toggle-row">
      <span className="toggle-label">{label}</span>
      <input
        type="checkbox"
        className="toggle-input"
        checked={checked}
        onChange={onToggle}
      />
      <span className="switch" aria-hidden="true" />
    </label>
  );
}

export default function SettingsPanel({
  settings,
  onChange,
  onClose,
}: {
  settings: AppSettings;
  onChange: (next: AppSettings) => void;
  onClose: () => void;
}) {
  const toggle = (key: ToggleKey) =>
    onChange({ ...settings, [key]: !settings[key] });

  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  useEffect(() => {
    listMonitors()
      .then(setMonitors)
      .catch(() => setMonitors([]));
  }, []);

  return (
    <div className="settings">
      <div className="settings-head" data-tauri-drag-region>
        <button className="back-btn" onClick={onClose} title="Zurück">
          ← Zurück
        </button>
        <span className="settings-title">Einstellungen</span>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Benachrichtigungen</div>
        {NOTIFY_TOGGLES.map((t) => (
          <ToggleRow
            key={t.key}
            label={t.label}
            checked={!!settings[t.key]}
            onToggle={() => toggle(t.key)}
          />
        ))}
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Allgemein</div>
        {GENERAL_TOGGLES.map((t) => (
          <ToggleRow
            key={t.key}
            label={t.label}
            checked={!!settings[t.key]}
            onToggle={() => toggle(t.key)}
          />
        ))}
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Statusleiste oben (Pill)</div>
        <ToggleRow
          label="Pill oben am Bildschirm anzeigen"
          checked={settings.islandEnabled}
          onToggle={() =>
            onChange({ ...settings, islandEnabled: !settings.islandEnabled })
          }
        />
        {settings.islandEnabled && (
          <label className="select-row">
            <span className="toggle-label">Bildschirm</span>
            <select
              className="settings-select"
              value={settings.islandMonitor ?? ""}
              onChange={(e) =>
                onChange({
                  ...settings,
                  islandMonitor: e.target.value === "" ? null : e.target.value,
                })
              }
            >
              <option value="">Primärmonitor</option>
              {monitors.map((m) => (
                <option key={m.name} value={m.name}>
                  {m.label}
                </option>
              ))}
            </select>
          </label>
        )}
        <div className="settings-note">
          Zeigt oben mittig eine gerundete Pill mit dem Live-Status aller
          Sessions (ein Punkt je Session, plus Zähler). Klick auf die Pill öffnet
          das Hauptfenster.
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Erweitert</div>
        <ToggleRow
          label="Rate-Limit-Anzeige (5h/7d) aktivieren"
          checked={settings.statuslineIntegration}
          onToggle={() =>
            onChange({
              ...settings,
              statuslineIntegration: !settings.statuslineIntegration,
            })
          }
        />
        <div className="settings-note">
          Schreibt eine <code>statusLine</code> in deine{" "}
          <code>~/.claude/settings.json</code>, um die 5h-/7d-Limits auszulesen.
          Vorher wird ein Backup angelegt (<code>settings.json.agentwatch-backup</code>);
          beim Deaktivieren wird alles wiederhergestellt. Wirkt erst in neu
          gestarteten Claude-Sessions.
        </div>
      </div>
    </div>
  );
}
