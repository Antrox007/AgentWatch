//! Gemeinsame Datenstrukturen, die als Snapshot ans Frontend gesendet werden.
//! Serialisierung erfolgt in camelCase, damit sie 1:1 zu den TypeScript-Typen passt.

use serde::{Deserialize, Serialize};

/// Zustand einer Session bzw. eines Subagents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// Agent arbeitet (Tools, Textgenerierung) — entspricht status "busy".
    Working,
    /// Agent wartet auf Benutzereingabe — entspricht status "waiting".
    Waiting,
    /// Agent ist idle/fertig — entspricht status "idle".
    Ready,
    /// Unbekannter Zustand.
    Unknown,
}

impl SessionState {
    /// Bildet das `status`-Feld aus sessions/<PID>.json auf einen Zustand ab.
    /// Beobachtete Werte: idle, busy, waiting, shell (laufender Shell-Command).
    pub fn from_status(status: &str) -> Self {
        match status {
            "busy" | "shell" => SessionState::Working,
            "waiting" => SessionState::Waiting,
            "idle" => SessionState::Ready,
            _ => SessionState::Unknown,
        }
    }

    /// Verfeinerte Ableitung fuer Sessions inkl. Aktivitaets-Signal.
    ///
    /// Der Marker-Status `shell` bleibt fuer die gesamte Laufzeit eines
    /// Shell-Befehls gesetzt — auch bei zurueckgelassenen Hintergrund-Prozessen
    /// (z.B. einem Dev-Server). Solch ein `shell` zaehlt daher nur dann als
    /// "arbeitet", wenn die Session kuerzlich Transcript-Aktivitaet zeigte;
    /// sonst gilt der Turn als abgeschlossen -> Ready. `busy` (LLM denkt/
    /// antwortet) zaehlt dagegen immer als "arbeitet".
    pub fn from_status_with_activity(status: &str, transcript_active: bool) -> Self {
        if status == "shell" && !transcript_active {
            SessionState::Ready
        } else {
            Self::from_status(status)
        }
    }
}

/// Ein einzelner Subagent einer Session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subagent {
    pub id: String,
    pub agent_type: String,
    pub state: SessionState,
}

/// Ein Punkt der Zustands-Historie einer Session (fuer die Timeline).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub t: i64,
    pub state: SessionState,
}

/// Eine aktive Claude-Code-Session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub pid: u32,
    pub cwd: String,
    pub project_name: String,
    pub git_branch: Option<String>,
    pub state: SessionState,
    pub model: Option<String>,
    /// Letzte Aktualisierung laut Heartbeat (Epoch-Millisekunden).
    pub updated_at: i64,
    pub started_at: Option<i64>,
    /// Aktuelle Kontextgroesse (Tokens der letzten Assistant-Nachricht).
    pub context_tokens: Option<u64>,
    pub context_window: Option<u64>,
    /// Kontextauslastung 0.0..1.0.
    pub context_utilization: Option<f64>,
    pub estimated_cost_usd: Option<f64>,
    /// True, wenn der Zustand `Ready` nur aus einem stehenden `shell` abgeleitet
    /// wurde (Marker NICHT `idle`). Solche Zustaende loesen keine
    /// "fertig"-Benachrichtigung aus — die kommt nur bei echtem Turn-Ende.
    pub idle_inferred: bool,
    /// Anzahl aktiver Subagents.
    pub subagent_count: usize,
    pub subagents: Vec<Subagent>,
    /// Letzter Assistant-Text (gekuerzt) — z.B. die Frage im Waiting-Zustand.
    pub last_text: Option<String>,
    /// Zustands-Historie (aelteste zuerst) — wird vom Watcher gefuellt.
    pub history: Vec<HistoryPoint>,
}

/// Gruppierung der Sessions nach Projekt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGroup {
    pub project_name: String,
    pub path: String,
    pub sessions: Vec<SessionInfo>,
}

/// Aggregierte Kennzahlen ueber alle Sessions.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub sessions: usize,
    pub working: usize,
    pub waiting: usize,
    pub ready: usize,
    pub agents: usize,
}

/// Account-weite Rate-Limits (Subscription 5h-/7d-Fenster), via opt-in statusLine.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimits {
    pub five_hour_pct: Option<f64>,
    pub seven_day_pct: Option<f64>,
    pub five_hour_resets_at: Option<String>,
    pub seven_day_resets_at: Option<String>,
}

/// Vollstaendiger Zustand zu einem Zeitpunkt — das, was das Frontend rendert.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub generated_at: i64,
    pub projects: Vec<ProjectGroup>,
    pub totals: Totals,
    pub rate_limits: Option<RateLimits>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping() {
        assert_eq!(SessionState::from_status("busy"), SessionState::Working);
        assert_eq!(SessionState::from_status("waiting"), SessionState::Waiting);
        assert_eq!(SessionState::from_status("idle"), SessionState::Ready);
        assert_eq!(SessionState::from_status("shell"), SessionState::Working);
        assert_eq!(SessionState::from_status("sonstwas"), SessionState::Unknown);
    }

    #[test]
    fn shell_requires_recent_activity_to_count_as_working() {
        // 'shell' mit frischer Aktivitaet -> arbeitet.
        assert_eq!(
            SessionState::from_status_with_activity("shell", true),
            SessionState::Working
        );
        // 'shell' ohne frische Aktivitaet (z.B. zurueckgelassener Dev-Server) -> bereit.
        assert_eq!(
            SessionState::from_status_with_activity("shell", false),
            SessionState::Ready
        );
        // 'busy' zaehlt unabhaengig vom Aktivitaets-Signal immer als arbeitet.
        assert_eq!(
            SessionState::from_status_with_activity("busy", false),
            SessionState::Working
        );
        // Andere Zustaende bleiben unveraendert.
        assert_eq!(
            SessionState::from_status_with_activity("waiting", false),
            SessionState::Waiting
        );
        assert_eq!(
            SessionState::from_status_with_activity("idle", false),
            SessionState::Ready
        );
    }
}
