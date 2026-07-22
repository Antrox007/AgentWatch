import { useEffect, useState } from "react";
import { listMonitors } from "./api";
import type { AppSettings, MonitorInfo } from "./types";
import { UpdateBanner } from "./UpdateBanner";
import type { UpdaterState } from "./useUpdater";

type ToggleKey = keyof Pick<
  AppSettings,
  | "notifyWaiting"
  | "notifyReady"
  | "notifyContextPressure"
  | "notifyNewSubagent"
  | "soundEnabled"
  | "showCost"
  | "autostart"
  | "autoUpdateCheck"
  | "islandShowEvents"
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
  updateState,
  onCheckForUpdates,
  onInstallUpdate,
  onDismissUpdate,
}: {
  settings: AppSettings;
  onChange: (next: AppSettings) => void;
  onClose: () => void;
  updateState: UpdaterState;
  onCheckForUpdates: () => void;
  onInstallUpdate: () => void;
  onDismissUpdate: () => void;
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
        <div className="settings-section-title">Windows-Benachrichtigungen (Toast)</div>
        <div className="settings-note">
          Diese Meldungen erscheinen als Windows-Popups rechts unten — unabhängig
          von der Pill. Standardmäßig aus, da die Pill den Status bereits visuell
          anzeigt.
        </div>
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
        <ToggleRow
          label="Ereignisse in der Pill aufklappen (fertig / wartet)"
          checked={settings.islandShowEvents}
          onToggle={() => toggle("islandShowEvents")}
        />
        <ToggleRow
          label="Rate-Limits dauerhaft in der Pill anzeigen (5h / 7d)"
          checked={settings.islandShowRateLimits}
          onToggle={() =>
            onChange({ ...settings, islandShowRateLimits: !settings.islandShowRateLimits })
          }
        />
        <div className="settings-note">
          Zeigt oben mittig eine gerundete Pill mit dem Live-Status aller
          Sessions (ein Punkt je Session). Bei aktiviertem Aufklappen erscheint
          kurz ein Texthinweis, wenn eine Session fertig wird oder auf Eingabe
          wartet. Klick auf die Pill öffnet das Hauptfenster.
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Updates</div>
        <ToggleRow
          label="Automatisch beim Start nach Updates suchen"
          checked={settings.autoUpdateCheck}
          onToggle={() =>
            onChange({ ...settings, autoUpdateCheck: !settings.autoUpdateCheck })
          }
        />
        <button className="check-updates-btn" onClick={onCheckForUpdates}>
          Jetzt nach Updates suchen
        </button>
        <UpdateBanner
          state={updateState}
          onInstall={onInstallUpdate}
          onDismiss={onDismissUpdate}
          className="update-banner-boxed"
        />
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
