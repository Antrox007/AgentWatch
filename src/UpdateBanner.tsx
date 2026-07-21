import type { UpdaterState } from "./useUpdater";

export function UpdateBanner({
  state,
  onInstall,
  onDismiss,
  className,
}: {
  state: UpdaterState;
  onInstall: () => void;
  onDismiss: () => void;
  /** Zusaetzliche Klasse fuer den Kontext (z.B. Settings-Karte statt Header-Strip). */
  className?: string;
}) {
  if (state.phase === "idle") return null;

  return (
    <div className={`update-banner${className ? ` ${className}` : ""}`}>
      {state.phase === "checking" && (
        <span className="update-text">Suche nach Updates …</span>
      )}

      {state.phase === "up-to-date" && (
        <span className="update-text">Du bist auf dem neuesten Stand</span>
      )}

      {state.phase === "available" && (
        <>
          <span className="update-text">
            Update <strong>v{state.version}</strong> verfügbar
          </span>
          <div className="update-actions">
            <button className="update-btn" onClick={onInstall}>
              Installieren
            </button>
            <button className="update-dismiss" onClick={onDismiss} title="Später">
              ✕
            </button>
          </div>
        </>
      )}

      {state.phase === "downloading" && (
        <>
          <span className="update-text">
            Wird heruntergeladen
            {state.progressPct != null ? ` … ${state.progressPct}%` : " …"}
          </span>
          <div className="update-progress">
            <div
              className="update-progress-fill"
              style={{ width: `${state.progressPct ?? 0}%` }}
            />
          </div>
        </>
      )}

      {state.phase === "restarting" && (
        <span className="update-text">Installiert — Neustart …</span>
      )}

      {state.phase === "error" && (
        <>
          <span className="update-text update-text-error">Update fehlgeschlagen</span>
          <div className="update-actions">
            <button className="update-btn" onClick={onInstall}>
              Erneut versuchen
            </button>
            <button className="update-dismiss" onClick={onDismiss} title="Schließen">
              ✕
            </button>
          </div>
        </>
      )}
    </div>
  );
}
